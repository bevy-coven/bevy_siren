"""Does `examples/semantic` rank the way `transformers` does, on the same real files?

The example prints `similarity` — CLAP's own cosine between the description and each asset. This computes the same number with
`transformers` directly and compares them, pair by pair, so a pipeline bug (a wrong mel, a wrong tokenizer, a wrong graph) cannot
hide behind "the model is just not very good at frogs".

    python3 parity.py <library-dir> "<phrase>" ["<phrase>" ...]       # run after the example has built the index

The reference uses `is_longer = false`, the path the crate runs (see `tools/export_clap_onnx.py`), not `get_audio_features`' default.
"""
import glob
import os
import re
import subprocess
import sys
import wave

import numpy as np
import torch
from transformers import ClapModel, ClapProcessor

MODEL = "laion/clap-htsat-fused"
EXAMPLE = os.path.join(os.path.dirname(os.path.abspath(__file__)), *[".."] * 6, "target", "release", "examples", "semantic")


def load(path):
    with wave.open(path) as w:
        data = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float32) / 32768.0
        return data.reshape(-1, w.getnchannels()).mean(axis=1), w.getframerate()


def unit(v):
    return torch.nn.functional.normalize(v, dim=-1)


def main():
    library, phrases = sys.argv[1], sys.argv[2:]
    model = ClapModel.from_pretrained(MODEL).eval()
    processor = ClapProcessor.from_pretrained(MODEL)

    audio = {}
    for path in sorted(glob.glob(os.path.join(library, "*.wav"))):
        samples, rate = load(path)
        feats = processor(audio=[samples], sampling_rate=rate, return_tensors="pt")["input_features"]
        with torch.no_grad():
            out = model.audio_model(input_features=feats, is_longer=torch.zeros(1, 1, dtype=torch.bool))
            audio[os.path.basename(path)] = unit(model.audio_projection(out.pooler_output))[0]

    worst = 0.0
    print("%-34s %-14s %9s %9s %9s" % ("phrase", "file", "crate", "reference", "diff"))
    for phrase in phrases:
        with torch.no_grad():
            tok = processor(text=[phrase], return_tensors="pt", padding=True)
            text = unit(model.get_text_features(**tok))
            text = text if isinstance(text, torch.Tensor) else text.pooler_output
        reference = {name: float(text[0] @ v) for name, v in audio.items()}

        run = subprocess.run(
            [EXAMPLE, library, phrase, "--k", str(len(audio)), "--no-play"], capture_output=True, text=True, env=os.environ.copy()
        )
        crate = {m.group(1): float(m.group(2)) for m in re.finditer(r"\d+\. (\S+\.wav)\s+score \S+\s+similarity (\S+)", run.stdout)}
        # The example's default floor is 0.0, so a negative cosine is *dropped* (`Tally::below_floor`), not returned. Check the
        # files it left out are exactly those, then compare every file it kept.
        left_out = set(reference) - set(crate)
        if not left_out <= {n for n, s in reference.items() if s < 0.0} or set(crate) - set(reference):
            sys.exit("the example's files and the reference's disagree: %s" % run.stdout[-400:])
        for name in sorted(crate, key=crate.get, reverse=True)[:3]:
            print("%-34s %-14s %9.4f %9.4f %9.1e" % (phrase, name, crate[name], reference[name], abs(crate[name] - reference[name])))
        diff = max(abs(crate[n] - reference[n]) for n in crate)
        worst = max(worst, diff)
        kept = [n for n in sorted(reference, key=reference.get, reverse=True) if n in crate]
        same_order = sorted(crate, key=crate.get, reverse=True) == kept
        print("   %d kept, %d below the floor; max |crate - reference| = %.2e; same ranking order: %s"
              % (len(crate), len(left_out), diff, same_order))
    print("\nworst difference over every phrase and file: %.2e" % worst)


if __name__ == "__main__":
    main()
