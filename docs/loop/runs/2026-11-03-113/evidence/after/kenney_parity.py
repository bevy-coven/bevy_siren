"""Does `examples/goldenbench --set kenney` agree with `transformers` — and does *how a short clip is padded* explain what it found?

    PYTHONPATH=/tmp/pydeps python3 kenney_parity.py <kenney-dir> <dump.tsv> <queries.tsv> [--report goldenbench-output.txt]

`dump.tsv` is `goldenbench --dump`: every description and every class prompt scored against every clip, by this crate. This script does three
independent things:

1. **The metric code.** zero-shot accuracy, hit@1/3/10, precision@10, MRR, mAP@10, the **separation** (the AUC of the top score, raw and
   centred on each query's own median, the weakest answerable top and the strongest unanswerable) and the floor tables are computed here, in
   Python, from the crate's own scores and compared with what the Rust printed. Two implementations that agree are the proof neither is a typo.
2. **The model path on game audio.** Every similarity the crate computed, against `transformers` on the same files (decoded by libsndfile:
   libvorbis for Ogg, mpg123 for MP3; mixed to mono; resampled by scipy's polyphase filter) with the extractor's own `repeatpad`.
3. **The padding experiment.** `ClapFeatureExtractor`'s default for audio shorter than the model's 10 s window is `repeatpad`: tile the clip
   as many whole times as fit, zero-pad the rest. A 0.28 s footstep becomes 35 footsteps in a row. The crate reproduces that exactly (check 2).
   The same files are embedded again with `padding="pad"` — the clip once, then silence — and every metric is computed again. That is
   **not what the model was trained with**, and is run to find out whether the loop is what makes one-shots hard to find, not to change anything.
"""
import argparse
import csv
import os
import re
import sys

import numpy as np
import soundfile as sf
import torch
from scipy.signal import resample_poly
from transformers import ClapModel, ClapProcessor

MODEL = "laion/clap-htsat-fused"
RATE = 48000
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), *[".."] * 6)
FLOORS = [0.25, 0.30, 0.35, 0.40, 0.45]
RELATIVE_FLOORS = [0.10, 0.20, 0.25, 0.30, 0.35, 0.40]


def unit(v):
    return torch.nn.functional.normalize(v, dim=-1)


def read_dump(path):
    with open(path, encoding="utf-8") as f:
        files = f.readline().rstrip("\n").split("\t")[1:]
        rows = {}
        for line in f:
            cells = line.rstrip("\n").split("\t")
            rows[cells[0]] = np.array([float(x) for x in cells[1:]], dtype=np.float64)
    return files, rows


def read_queries(path):
    out = [l.rstrip("\n").split("\t") for l in open(path, encoding="utf-8") if l.strip() and not l.startswith("#")]
    header, rows = out[0], out[1:]
    c, d = header.index("class"), header.index("description")
    return [(r[d], None if r[c] == "-" else r[c]) for r in rows]


def ranked(scores):
    return np.lexsort((np.arange(len(scores)), -scores))  # best first, ties by position


def metrics(matrix, files, truth, queries, prompts):
    """Zero-shot accuracy and the retrieval metrics from a {string: scores over files} matrix."""
    classes = sorted(set(truth))
    prompt_rows = np.stack([matrix[prompts[c]] for c in classes])
    zero_shot = float((np.array(classes)[prompt_rows.argmax(axis=0)] == truth).mean())
    hit = {1: [], 3: [], 10: []}
    p10, rr, ap, first_rank = [], [], [], {}
    for description, cls in queries:
        if cls is None:
            continue
        right = truth[ranked(matrix[description])] == cls
        for k in hit:
            hit[k].append(right[:k].any())
        p10.append(right[:10].mean())
        first = np.flatnonzero(right)
        first_rank[description] = int(first[0]) + 1 if len(first) else None
        rr.append(1.0 / (first[0] + 1) if len(first) else 0.0)
        possible = min(10, int((truth == cls).sum()))
        hits, total = 0, 0.0
        for i, r in enumerate(right[:10]):
            if r:
                hits += 1
                total += hits / (i + 1)
        ap.append(total / possible)
    return {
        "zero_shot": zero_shot, "hit_1": float(np.mean(hit[1])), "hit_3": float(np.mean(hit[3])), "hit_10": float(np.mean(hit[10])),
        "precision_10": float(np.mean(p10)), "mrr": float(np.mean(rr)), "map_10": float(np.mean(ap)),
    }, first_rank


def auc(positive, negative):
    wins = sum(1.0 if p > n else 0.5 if p == n else 0.0 for p in positive for n in negative)
    return wins / (len(positive) * len(negative))


def median_rank(s):
    """The crate's median: nearest rank, `sorted[round((n - 1) * 0.5)]` with a half rounded up — the upper middle for an even count, and a score that occurred."""
    return np.sort(s)[int(np.floor((len(s) - 1) * 0.5 + 0.5))]


def separation(matrix, queries, centre):
    """Top score of each query (optionally less that query's own median over the library): AUC, weakest answerable, strongest unanswerable."""
    def top(description):
        s = matrix[description]
        return float(s.max() - (median_rank(s) if centre else 0.0))
    a = [top(d) for d, c in queries if c is not None]
    n = [top(d) for d, c in queries if c is None]
    return auc(a, n), min(a), max(n)


def floor_rows(matrix, truth, queries, floors, centre):
    """The crate's `FloorRow`s, recomputed: top-1 kept, hit@3 kept, returned, precision, silent."""
    rows = []
    for floor in floors:
        top1 = hit3 = returned = right_total = 0
        positives = [(d, c) for d, c in queries if c is not None]
        negatives = [d for d, c in queries if c is None]
        for d, c in positives:
            s = matrix[d] - (median_rank(matrix[d]) if centre else 0.0)
            order = ranked(s)
            n = int((s[order] >= floor).sum())
            returned += n
            right = truth[order] == c
            right_total += int(right[:n].sum())
            top1 += int(n > 0 and right[0])
            hit3 += int(right[:min(n, 3)].any())
        silent = np.mean([(matrix[d] - (median_rank(matrix[d]) if centre else 0.0)).max() < floor for d in negatives])
        q = len(positives)
        rows.append((floor, top1 / q, hit3 / q, returned / q, right_total / returned if returned else None, float(silent)))
    return rows


def parse_report(text):
    printed = {}
    printed["zero_shot"] = float(re.search(r"accuracy\s+([\d.]+) %", text).group(1)) / 100
    m = re.search(r"hit@1\s+([\d.]+) %\s+hit@3\s+([\d.]+) %\s+hit@10\s+([\d.]+) %\s+precision@10\s+([\d.]+) %\s+MRR ([\d.]+)\s+mAP@10 ([\d.]+)", text)
    for name, value, scale in zip(["hit_1", "hit_3", "hit_10", "precision_10", "mrr", "map_10"], m.groups(), [100, 100, 100, 100, 1, 1]):
        printed[name] = float(value) / scale
    sep = re.search(r"the cosine itself\s+([\d.]+)\s+([+-][\d.]+)\s+([+-][\d.]+)\s+(\w+)\s*\n\s+less the query's own median\s+([\d.]+)\s+([+-][\d.]+)\s+([+-][\d.]+)", text)
    printed["sep_raw"] = tuple(float(sep.group(i)) for i in (1, 2, 3))
    printed["sep_relative"] = tuple(float(sep.group(i)) for i in (5, 6, 7))
    tables = re.split(r"4b\. ", text)
    row = r"^\s+([+-]\d\.\d\d)\s+([\d.]+) %\s+([\d.]+) %\s+([\d.]+)\s+([\d.]+ %|—)\s+([\d.]+) %"
    parse = lambda part: {float(m.group(1)): (float(m.group(2)) / 100, float(m.group(3)) / 100, float(m.group(4)),
                                              None if m.group(5) == "—" else float(m.group(5).split()[0]) / 100, float(m.group(6)) / 100)
                          for m in re.finditer(row, part, re.M)}
    printed["floors_raw"], printed["floors_relative"] = parse(tables[0].split("4. The floor")[1]), parse(tables[1])
    return printed


def load_clip(path):
    """The clip as the reference reads it: libsndfile, mixed to mono, resampled to 48 kHz by scipy's polyphase filter. -> (samples, seconds, rate)"""
    data, rate = sf.read(path, dtype="float32", always_2d=True)
    mono = data.mean(axis=1)
    if rate != RATE:
        g = np.gcd(rate, RATE)
        mono = resample_poly(mono, RATE // g, rate // g).astype(np.float32)
    return mono, len(data) / rate, rate


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dir")
    parser.add_argument("dump")
    parser.add_argument("queries")
    parser.add_argument("--report", help="goldenbench's printed output, to compare its numbers with the Python ones")
    parser.add_argument("--limit", type=int, default=0, help="the model path (checks 2 and 3) on every n-th clip only, for a quicker look; check 1 always runs on every clip")
    args = parser.parse_args()

    files, crate = read_dump(args.dump)
    labels = {}
    with open(os.path.join(args.dir, "labels.csv"), newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            labels[row["file"]] = row["class"]
    queries = read_queries(args.queries)
    prompts = {c: "This is a sound of %s." % c.replace("_", " ") for c in sorted(set(labels.values()))}
    truth = np.array([labels[f] for f in files])

    # --- 1. the metric code, on the crate's own scores
    ours, _ = metrics(crate, files, truth, queries, prompts)
    sep_raw, sep_rel = separation(crate, queries, False), separation(crate, queries, True)
    raw_rows, rel_rows = floor_rows(crate, truth, queries, FLOORS, False), floor_rows(crate, truth, queries, RELATIVE_FLOORS, True)
    print("1. THE METRIC CODE — from the crate's scores, computed in Python")
    for k, v in ours.items():
        print("   %-13s %.4f" % (k, v))
    print("   separation, raw cosine:        AUC %.3f   weakest answerable %+.3f   strongest unanswerable %+.3f" % sep_raw)
    print("   separation, less query median: AUC %.3f   weakest answerable %+.3f   strongest unanswerable %+.3f" % sep_rel)
    if args.report:
        printed = parse_report(open(args.report, encoding="utf-8").read())
        worst = max(abs(printed[k] - ours[k]) for k in ours)
        print("   Rust's printed metrics vs Python: largest difference %.4f over %d metrics -> %s" % (worst, len(ours), "agree" if worst < 6e-4 else "DISAGREE"))
        d_raw = max(abs(a - b) for a, b in zip(printed["sep_raw"], sep_raw))
        d_rel = max(abs(a - b) for a, b in zip(printed["sep_relative"], sep_rel))
        print("   Rust's printed separation vs Python: largest difference %.4f (raw), %.4f (relative) -> %s" % (d_raw, d_rel, "agree" if max(d_raw, d_rel) < 6e-4 else "DISAGREE"))
        # A percentage is printed to a tenth of a point and `returned` to a tenth of a result, so each column has its own tolerance.
        tolerance = (6e-4, 6e-4, 0.06, 6e-4, 6e-4)
        mismatched, cells = [], 0
        for table, rows in (("floors_raw", raw_rows), ("floors_relative", rel_rows)):
            for floor, *values in rows:
                theirs = printed[table].get(round(floor, 2))
                if theirs is None:
                    continue
                for a, b, tol in zip(theirs, values, tolerance):
                    cells += 1
                    if (a is None) != (b is None) or (a is not None and abs(a - b) > tol):
                        mismatched.append((table, floor, a, b))
        print("   Rust's printed floor tables vs Python: %d cells, %d differ -> %s" % (cells, len(mismatched), "agree to the printed precision" if not mismatched else "DISAGREE %s" % mismatched[:3]))

    # --- 2 and 3. the model path, from the audio on disk, with each padding
    step = args.limit or 1
    chosen = list(range(0, len(files), step))
    model = ClapModel.from_pretrained(MODEL).eval()
    processor = ClapProcessor.from_pretrained(MODEL)
    strings = [d for d, _ in queries] + list(prompts.values())
    with torch.no_grad():
        text = unit(model.get_text_features(**processor(text=strings, return_tensors="pt", padding=True)))
    not_longer = torch.zeros(1, 1, dtype=torch.bool)
    embeddings = {"repeatpad": [], "pad": []}
    seconds, rates = [], []
    for n, i in enumerate(chosen):
        samples, length, rate = load_clip(os.path.join(args.dir, files[i]))
        seconds.append(length)
        rates.append(rate)
        for mode in embeddings:
            feats = processor.feature_extractor(samples, sampling_rate=RATE, padding=mode, return_tensors="pt")["input_features"]
            if n == 0 and mode == "repeatpad":
                same = processor(audio=[samples], sampling_rate=RATE, return_tensors="pt")["input_features"]
                assert torch.allclose(feats, same), "the feature extractor and the processor disagree on repeatpad"
            with torch.no_grad():
                out = model.audio_model(input_features=feats, is_longer=not_longer)
                embeddings[mode].append(unit(model.audio_projection(out.pooler_output))[0])
        if n % 50 == 0:
            print("   reference embeddings: %d / %d" % (n, len(chosen)), file=sys.stderr)
    seconds, rates = np.array(seconds), np.array(rates)
    reference = {mode: {s: (text[j] @ torch.stack(e).T).numpy().astype(np.float64) for j, s in enumerate(strings)} for mode, e in embeddings.items()}
    sub_files, sub_truth = [files[i] for i in chosen], truth[chosen]

    if step == 1:
        diff = np.concatenate([np.abs(reference["repeatpad"][s] - crate[s]) for s in strings])
        per_clip = np.mean([np.abs(reference["repeatpad"][s] - crate[s]) for s in strings], axis=0)
        print("\n2. THE MODEL PATH — the crate against transformers (extractor's repeatpad), %d clips x %d strings = %d pairs" % (len(chosen), len(strings), diff.size))
        print("   max |difference| %.2e   mean %.2e   99.9th percentile %.2e" % (diff.max(), diff.mean(), np.percentile(diff, 99.9)))
        print("   per clip (mean |d| over the strings): median %.1e, 99th percentile %.1e, max %.1e" % (np.median(per_clip), np.percentile(per_clip, 99), per_clip.max()))
        print("   the five clips the two pipelines disagree on most:")
        for j in np.argsort(per_clip)[::-1][:5]:
            print("     %-40s mean |d| %.3f   %.2f s   class %s" % (files[chosen[j]], per_clip[j], seconds[j], truth[chosen[j]]))
        print("   where the difference is — mean |d| over the strings, by each file's own sample rate and length:")
        for label, mask in (
            ("48 kHz files", rates == 48000),
            ("44.1 kHz files (resampled by both, differently)", rates == 44100),
            ("under 1 s", seconds < 1.0),
            ("1 s or longer", seconds >= 1.0),
            ("44.1 kHz and under 1 s", (rates == 44100) & (seconds < 1.0)),
            ("44.1 kHz and 1 s or longer", (rates == 44100) & (seconds >= 1.0)),
        ):
            if mask.any():
                print("     %-48s %3d files   median %.1e   mean %.1e   max %.1e" % (label, mask.sum(), np.median(per_clip[mask]), per_clip[mask].mean(), per_clip[mask].max()))
        theirs, _ = metrics(reference["repeatpad"], sub_files, sub_truth, queries, prompts)
        print("   the same metrics from transformers' similarities:")
        for k in theirs:
            print("     %-13s transformers %.4f   crate %.4f   difference %+.4f" % (k, theirs[k], ours[k], ours[k] - theirs[k]))

    print("\n3. THE PADDING EXPERIMENT — the same clips, the same questions, the clip padded two ways (clips: %d, durations %.2f - %.2f s, median %.2f s)"
          % (len(chosen), seconds.min(), seconds.max(), np.median(seconds)))
    results = {}
    for mode, label in (("repeatpad", "repeatpad (the model's own, and the crate's)"), ("pad", "pad (the clip once, then silence)")):
        m, first = metrics(reference[mode], sub_files, sub_truth, queries, prompts)
        raw_sep, rel_sep = separation(reference[mode], queries, False), separation(reference[mode], queries, True)
        right = np.concatenate([reference[mode][d][sub_truth == c] for d, c in queries if c is not None])
        results[mode] = (m, first, raw_sep, rel_sep, right)
        print("   %s" % label)
        print("     zero-shot %.3f   hit@1 %.3f   hit@3 %.3f   hit@10 %.3f   MRR %.3f   mAP@10 %.3f" % (m["zero_shot"], m["hit_1"], m["hit_3"], m["hit_10"], m["mrr"], m["map_10"]))
        print("     right answers' median similarity %+.3f;  separation: AUC %.3f (weakest answerable %+.3f, strongest unanswerable %+.3f);  centred: AUC %.3f" % (np.median(right), raw_sep[0], raw_sep[1], raw_sep[2], rel_sep[0]))
    print("\n   first right answer's rank, query by query (the maintainer's wording):")
    print("     %-44s %10s %10s" % ("description", "repeatpad", "pad"))
    for d, c in queries:
        if c is not None:
            print("     %-44s %10s %10s" % (d[:44], results["repeatpad"][1][d], results["pad"][1][d]))
    short = seconds < 1.0
    print("\n   clips under one second: %d of %d. Their right-answer similarity, median: repeatpad %+.3f, pad %+.3f" % (
        short.sum(), len(chosen),
        np.median(np.concatenate([reference["repeatpad"][d][(sub_truth == c) & short] for d, c in queries if c is not None and ((sub_truth == c) & short).any()])),
        np.median(np.concatenate([reference["pad"][d][(sub_truth == c) & short] for d, c in queries if c is not None and ((sub_truth == c) & short).any()]))))


if __name__ == "__main__":
    main()
