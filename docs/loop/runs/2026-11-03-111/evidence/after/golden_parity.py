"""Does `examples/goldenbench` agree with `transformers` on the real labelled set — including 44.1 kHz input?

    python3 golden_parity.py <esc50-dir> <dump.tsv> [--report goldenbench-output.txt] [--limit N]

`dump.tsv` is `goldenbench --dump`: every description and every class prompt scored against every clip, by this crate. This
script recomputes the same similarities with `transformers` on the path the crate runs (`audio_model(is_longer=False)`), and does
two independent checks:

1. **The model path, at 44.1 kHz.** Every similarity the crate computed against the one `transformers` computes, pair by pair.
   ESC-50 is 44.1 kHz and the crate resamples it with `rubato`; here it is resampled with scipy's polyphase filter (160/147), so a
   difference is a difference between two resamplers on top of everything else. Iteration 110's 5 × 14 check was at 48 kHz and
   could not see this.
2. **The metric code.** zero-shot accuracy, hit@1/3/10, precision@10, MRR and mAP@10 are computed here, in Python, from the crate's own
   scores — and compared with what the Rust prints. Two implementations of one metric that agree are the proof that neither
   is a typo.
"""
import argparse
import csv
import os
import re
import sys
import wave

import numpy as np
import torch
from scipy.signal import resample_poly
from transformers import ClapModel, ClapProcessor

MODEL = "laion/clap-htsat-fused"
RATE = 48000
HERE = os.path.dirname(os.path.abspath(__file__))
QUERIES = os.path.join(HERE, *[".."] * 6, "golden", "esc50", "queries.tsv")


def load(path):
    with wave.open(path) as w:
        data = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float32) / 32768.0
        return data.reshape(-1, w.getnchannels()).mean(axis=1), w.getframerate()


def unit(v):
    return torch.nn.functional.normalize(v, dim=-1)


def read_dump(path):
    with open(path, encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        files = header[1:]
        rows = {}
        for line in f:
            cells = line.rstrip("\n").split("\t")
            rows[cells[0]] = np.array([float(x) for x in cells[1:]], dtype=np.float64)
    return files, rows


def read_queries():
    out = []
    for line in open(QUERIES, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cells = line.rstrip("\n").split("\t")
        out.append(cells)
    header, rows = out[0], out[1:]
    c, d = header.index("class"), header.index("description")
    return [(r[d], None if r[c] == "-" else r[c]) for r in rows]


def metrics(matrix, descriptions, files, labels, queries, prompts):
    """zero-shot accuracy and the retrieval metrics from a {description: scores-over-files} matrix."""
    truth = np.array([labels[f] for f in files])
    classes = sorted(set(truth))
    # zero-shot: each clip against the class prompts
    prompt_rows = np.stack([matrix[prompts[c]] for c in classes])  # (classes, clips)
    predicted = np.array(classes)[prompt_rows.argmax(axis=0)]
    zero_shot = float((predicted == truth).mean())
    hit = {1: [], 3: [], 10: []}
    p10, rr, ap = [], [], []
    for description, cls in queries:
        if cls is None:
            continue
        scores = matrix[description]
        order = np.lexsort((np.arange(len(files)), -scores))  # best first, ties by position (the crate breaks ties by hash)
        right = (truth[order] == cls)
        for k in hit:
            hit[k].append(right[:k].any())
        p10.append(right[:10].mean())
        first = np.flatnonzero(right)
        rr.append(1.0 / (first[0] + 1) if len(first) else 0.0)
        possible = min(10, int((truth == cls).sum()))
        hits, total = 0, 0.0
        for i, r in enumerate(right[:10]):
            if r:
                hits += 1
                total += hits / (i + 1)
        ap.append(total / possible)
    return {
        "zero_shot": zero_shot,
        "hit_1": float(np.mean(hit[1])), "hit_3": float(np.mean(hit[3])), "hit_10": float(np.mean(hit[10])),
        "precision_10": float(np.mean(p10)), "mrr": float(np.mean(rr)), "map_10": float(np.mean(ap)),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dir")
    parser.add_argument("dump")
    parser.add_argument("--report", help="goldenbench's printed output, to compare its metrics with the Python ones")
    parser.add_argument("--limit", type=int, default=0, help="every n-th clip only, for a quicker look")
    args = parser.parse_args()

    files, crate = read_dump(args.dump)
    labels = {}
    with open(os.path.join(args.dir, "labels.csv"), newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            labels[row["file"]] = row["class"]
    queries = read_queries()
    prompts = {c: "This is a sound of %s." % c.replace("_", " ") for c in sorted(set(labels.values()))}

    # --- 2. the metric code, on the crate's own scores
    ours = metrics(crate, None, files, labels, queries, prompts)
    print("metrics from the crate's scores, computed in Python:")
    for k, v in ours.items():
        print("  %-13s %.4f" % (k, v))
    if args.report:
        text = open(args.report, encoding="utf-8").read()
        printed = {}
        m = re.search(r"accuracy\s+([\d.]+) %", text)
        printed["zero_shot"] = float(m.group(1)) / 100
        m = re.search(r"hit@1\s+([\d.]+) %\s+hit@3\s+([\d.]+) %\s+hit@10\s+([\d.]+) %\s+precision@10\s+([\d.]+) %\s+MRR ([\d.]+)\s+mAP@10 ([\d.]+)", text)
        for name, value, scale in zip(["hit_1", "hit_3", "hit_10", "precision_10", "mrr", "map_10"], m.groups(), [100, 100, 100, 100, 1, 1]):
            printed[name] = float(value) / scale
        worst = max(abs(printed[k] - ours[k]) for k in printed)
        print("Rust prints (rounded) vs Python: largest difference %.4f over %d metrics -> %s"
              % (worst, len(printed), "agree to the printed precision" if worst < 6e-4 else "DISAGREE"))
        for k in printed:
            print("  %-13s rust %.4f  python %.4f" % (k, printed[k], ours[k]))

    # --- 1. the model path, from the audio on disk
    step = args.limit or 1
    chosen = list(range(0, len(files), step))
    model = ClapModel.from_pretrained(MODEL).eval()
    processor = ClapProcessor.from_pretrained(MODEL)
    strings = [d for d, _ in queries] + list(prompts.values())
    with torch.no_grad():
        text = unit(model.get_text_features(**processor(text=strings, return_tensors="pt", padding=True)))
    embeddings, zero_fraction = [], []
    not_longer = torch.zeros(1, 1, dtype=torch.bool)
    for n, i in enumerate(chosen):
        samples, rate = load(os.path.join(args.dir, files[i]))
        zero_fraction.append(float((samples == 0).mean()))  # ESC-50 pads a short clip to 5 s with digital silence
        if rate != RATE:
            g = np.gcd(rate, RATE)
            samples = resample_poly(samples, RATE // g, rate // g).astype(np.float32)
        feats = processor(audio=[samples], sampling_rate=RATE, return_tensors="pt")["input_features"]
        with torch.no_grad():
            out = model.audio_model(input_features=feats, is_longer=not_longer)
            embeddings.append(unit(model.audio_projection(out.pooler_output))[0])
        if n % 250 == 0:
            print("  reference embeddings: %d / %d" % (n, len(chosen)), file=sys.stderr)
    audio = torch.stack(embeddings)
    reference = {s: (text[j] @ audio.T).numpy().astype(np.float64) for j, s in enumerate(strings)}

    differences = np.concatenate([np.abs(reference[s] - crate[s][chosen]) for s in strings])
    print("\nsimilarities over %d clips x %d strings = %d pairs, crate (rubato) vs transformers (scipy resampling):"
          % (len(chosen), len(strings), differences.size))
    print("  max |difference| %.2e   mean %.2e   99.9th percentile %.2e" % (differences.max(), differences.mean(), np.percentile(differences, 99.9)))
    per_clip = np.mean([np.abs(reference[s] - crate[s][chosen]) for s in strings], axis=0)
    from scipy.stats import spearmanr
    rho = spearmanr(per_clip, zero_fraction).correlation
    print("\nper clip (mean |difference| over the %d strings): median %.1e, 99th percentile %.1e, max %.1e" % (len(strings), np.median(per_clip), np.percentile(per_clip, 99), per_clip.max()))
    print("Spearman correlation of a clip's difference with its share of exactly-zero samples (digital silence): %+.2f" % rho)
    print("the ten clips the two pipelines disagree on most:")
    for j in np.argsort(per_clip)[::-1][:10]:
        print("  %-34s mean |d| %.3f   zero samples %4.0f %%   class %s" % (files[chosen[j]], per_clip[j], zero_fraction[j] * 100, labels[files[chosen[j]]]))
    sub_files = [files[i] for i in chosen]
    theirs = metrics(reference, None, sub_files, labels, queries, prompts)
    sub_crate = {s: crate[s][chosen] for s in strings}
    mine = metrics(sub_crate, None, sub_files, labels, queries, prompts)
    print("\nthe same metrics from the reference's similarities (clips: %d):" % len(chosen))
    for k in theirs:
        print("  %-13s reference %.4f   crate %.4f   difference %+.4f" % (k, theirs[k], mine[k], mine[k] - theirs[k]))


if __name__ == "__main__":
    main()
