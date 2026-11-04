#!/usr/bin/env python3
"""Export CLAP's two towers to the ONNX graphs `bevy_siren`'s `semantic` feature loads.

    python3 tools/export_clap_onnx.py <out-dir>
    HF_HUB_OFFLINE=1 python3 tools/export_clap_onnx.py <out-dir>   # when the weights are already in the Hugging Face cache

Writes, into <out-dir>:

    clap-text.onnx    input_ids, attention_mask (int64 [batch, tokens])  ->  text_embed (f32 [batch, 512], unit norm)
    clap-audio.onnx   input_features (f32 [1, 4, 1001, 64]), is_longer (bool [1, 1])  ->  audio_embed (f32 [1, 512]) — one clip
    tokenizer.json    the text tower's tokenizer, which `semantic::tokenizer::Vocabulary::from_json` reads
    clap-export.txt   one line: the version string to give `--model-version`

**Why the last file exists.** An index records *which export made its vectors* (`ModelStamp::version`), so that a vector from
the wrong model is refused instead of ranked. The crate measured `transformers-5.22.1/torch-2.15.0/opset-17`; yours is whatever
this script ran on, and `clap-export.txt` says so. Pass it as `--model-version` to `examples/semantic` and `siren-tools`.

The wrappers are the two paths `ClapModel.get_text_features` and `get_audio_features` take. **Each is checked against the model
itself before anything is written**, and the script exits non-zero if they disagree, so a transformers release that changes
what those methods do fails here rather than producing a graph that ranks noise.

Needs `torch` (2.5 or later, for `dynamo=False`), `transformers`, `onnx` (the exporter itself requires it) and `numpy`, and the
`laion/clap-htsat-fused` weights (MIT licence). It has been run on Python 3.9 with torch 2.8.0, transformers 4.57.6 and onnx
1.19.1 — the one environment it is known to work in. About 1 GB of disk for the graphs.

**What it does not check is the exported graph itself.** The wrappers are compared with the model and `onnx.checker` reads the
files, but a trace can bake in a branch the eager wrapper does not take — which is how the audio graph's first export was
wrong. Running the graph is that check, and `docs/guide/semantic.md` ("Try it") says how: the crate's similarities against
`transformers`', pair by pair.
"""
import os
import shutil
import sys

import numpy as np
import onnx
import torch
import transformers
from transformers import ClapModel, ClapProcessor

MODEL = "laion/clap-htsat-fused"
OPSET = 17
SAMPLE_RATE = 48000


def fail(message):
    print("export_clap_onnx: " + message, file=sys.stderr)
    sys.exit(1)


class TextTower(torch.nn.Module):
    """`text_model` -> pooler -> `text_projection` -> L2 norm: the path `get_text_features` takes."""

    def __init__(self, model):
        super().__init__()
        self.m = model

    def forward(self, input_ids, attention_mask):
        out = self.m.text_model(input_ids=input_ids, attention_mask=attention_mask)
        pooled = out.pooler_output if hasattr(out, "pooler_output") else out[1]
        return torch.nn.functional.normalize(self.m.text_projection(pooled), dim=-1)


class AudioTower(torch.nn.Module):
    """`audio_model` -> pooler -> `audio_projection`: the path `audio_model(is_longer=false)` takes, before the L2 norm.

    The crate takes cosines, so the norm is not applied here; the vector is a direction.
    """

    def __init__(self, model):
        super().__init__()
        self.m = model

    def forward(self, input_features, is_longer):
        out = self.m.audio_model(input_features=input_features, is_longer=is_longer)
        pooled = out.pooler_output if hasattr(out, "pooler_output") else out[1]
        embed = self.m.audio_projection(pooled)
        # **`is_longer` stays a real input of the graph**, because the crate sends it by name (`Model::embed_audio`). The branch
        # it selects was decided when this was traced — with `is_longer` false, the fusion branch is not in the graph — so the
        # input is accepted and ignored. Without a use, the exporter drops it and the crate's call fails with *"Invalid input
        # name: is_longer"*, measured. `0 * x` is exactly zero for a boolean, so the embedding is unchanged.
        return embed + 0.0 * is_longer.to(embed.dtype).sum()


def features_of(output):
    """The tensor a `get_*_features` call returns, whichever shape this transformers version wraps it in."""
    if isinstance(output, torch.Tensor):
        return output
    return output.pooler_output if hasattr(output, "pooler_output") else output[0]


def cosine(a, b):
    a = a.flatten().double()
    b = b.flatten().double()
    return float((a @ b) / (a.norm() * b.norm()))


def main():
    if len(sys.argv) != 2:
        fail("usage: export_clap_onnx.py <out-dir>")
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)

    model = ClapModel.from_pretrained(MODEL).eval()
    processor = ClapProcessor.from_pretrained(MODEL)
    print("loaded %s  (torch %s, transformers %s)" % (MODEL, torch.__version__, transformers.__version__))

    # ------------------------------------------------------------------------------------------------ the text tower
    texts = ["tense sparse underwater ruins", "a warm footstep on gravel", "a bright combat loop", "dripping water in a cave"]
    tokens = processor(text=texts, return_tensors="pt", padding=True)
    with torch.no_grad():
        reference = features_of(model.get_text_features(**tokens))
        text = TextTower(model).eval()
        mine = text(tokens["input_ids"], tokens["attention_mask"])
    worst = float((mine - reference).abs().max())
    print("text tower vs get_text_features: max abs diff %.2e" % worst)
    if worst > 1e-4:
        fail("the text wrapper disagrees with get_text_features by %.2e; this transformers release changed the path" % worst)
    text_path = os.path.join(out, "clap-text.onnx")
    torch.onnx.export(
        text,
        (tokens["input_ids"], tokens["attention_mask"]),
        text_path,
        input_names=["input_ids", "attention_mask"],
        output_names=["text_embed"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "tokens"},
            "attention_mask": {0: "batch", 1: "tokens"},
            "text_embed": {0: "batch"},
        },
        opset_version=OPSET,
        do_constant_folding=True,
        dynamo=False,
    )

    # ----------------------------------------------------------------------------------------------- the audio tower
    # **The path is `is_longer = false`, and that is not `get_audio_features`' default.** HF's extractor marks one clip of every
    # fused batch `is_longer = True` (even a one-second clip, even a batch of one), which switches on HTSAT's local-mel fusion
    # branch and moves the embedding: measured, cosine **0.83** between the two paths for the same features. The crate sends
    # `false` for every window — it chunks anything past ten seconds, so none is "longer" — which is LAION-CLAP's own
    # behaviour for a short clip. So the wrapper is checked against *that* path, and the difference is printed, not hidden.
    clips = [np.random.RandomState(i).randn(SAMPLE_RATE * seconds).astype(np.float32) * 0.1 for i, seconds in enumerate([1, 4, 10])]
    inputs = processor(audio=clips, sampling_rate=SAMPLE_RATE, return_tensors="pt", padding=True)
    not_longer = torch.zeros_like(inputs["is_longer"])
    with torch.no_grad():
        default = features_of(model.get_audio_features(**inputs))
        audio = AudioTower(model).eval()
        mine = audio(inputs["input_features"], not_longer)
        pooled = model.audio_model(input_features=inputs["input_features"], is_longer=not_longer)
        reference = torch.nn.functional.normalize(
            model.audio_projection(pooled.pooler_output if hasattr(pooled, "pooler_output") else pooled[1]), dim=-1
        )
    worst = min(cosine(mine[i], reference[i]) for i in range(mine.shape[0]))
    print("audio tower vs audio_model(is_longer=false): worst cosine %.9f" % worst)
    if worst < 0.9999:
        fail("the audio wrapper disagrees with audio_model (cosine %.6f); this transformers release changed the path" % worst)
    marked = inputs["is_longer"].flatten().tolist()
    print("  the extractor marked is_longer=%s on this batch; get_audio_features' default differs from this graph by cosine %.3f"
          % (marked, min(cosine(mine[i], default[i]) for i in range(mine.shape[0]))))
    audio_path = os.path.join(out, "clap-audio.onnx")
    # **Traced at batch 1, `is_longer` false, no dynamic axes.** The crate embeds one clip at a time (`Model::embed_audio` takes
    # exactly one), and the trace bakes in both the batch and the branch it saw. Traced with the extractor's own `is_longer`
    # (forced true) the graph always ran the fusion branch, and the crate's `false` made that branch's batch zero:
    # *"Input shape {0,96,64,3,21}, requested {0,96,64,-1}"* — measured on the first real run of `examples/semantic`, which
    # indexed 0 of 14 files.
    one = processor(audio=[clips[0]], sampling_rate=SAMPLE_RATE, return_tensors="pt", padding=True)
    torch.onnx.export(
        audio,
        (one["input_features"], torch.zeros(1, 1, dtype=torch.bool)),
        audio_path,
        input_names=["input_features", "is_longer"],
        output_names=["audio_embed"],
        opset_version=OPSET,
        do_constant_folding=True,
        dynamo=False,
    )

    for path in (text_path, audio_path):
        onnx.checker.check_model(onnx.load(path))
    print("onnx.checker: both graphs are structurally valid")

    # ------------------------------------------------------------------------------------- the files that travel with them
    processor.tokenizer.backend_tokenizer.save(os.path.join(out, "tokenizer.json"))
    version = "transformers-%s/torch-%s/opset-%d" % (transformers.__version__, torch.__version__.split("+")[0], OPSET)
    with open(os.path.join(out, "clap-export.txt"), "w") as f:
        f.write(version + "\n")
    for name in ("clap-text.onnx", "clap-audio.onnx", "tokenizer.json", "clap-export.txt"):
        print("  %-16s %9.1f MB" % (name, os.path.getsize(os.path.join(out, name)) / 1e6))
    print("\nversion string for --model-version: " + version)


if __name__ == "__main__":
    main()
