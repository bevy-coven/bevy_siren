"""How sure is the separation? A bootstrap over the *queries* of each set, in Python, on the crate's own scores. Nothing here is in the crate.

    python3 bootstrap.py <dump.tsv> <queries.tsv> [--resamples 4000] [--seed 113]

`dump.tsv` is `goldenbench --dump`. A set has 29-50 answerable and 16-18 unanswerable descriptions, so an AUC is an average over 400-800 pairs of
*descriptions*, not of independent measurements: the descriptions are the sample. Each resample draws the answerable queries with replacement
and the unanswerable ones with replacement, recomputes the AUC of the top score (ties half) on the raw cosines and on scores centred on each
query's own median (nearest rank, the crate's `eval::centred`), and keeps the **paired difference**, because the two AUCs are computed on the
same resample. The share of answerable descriptions whose best result is under a floor is a proportion of descriptions, so it gets a Wilson
interval, which at 0 of 50 is not [0, 0].

This says how far the *descriptions* could have moved the answer. It does not say how the answer would move with a different library, a third
writer's words or another model: those are the limits `docs/guide/golden-set.md` already names.
"""
import argparse

import numpy as np


def read_dump(path):
    with open(path, encoding="utf-8") as f:
        files = f.readline().rstrip("\n").split("\t")[1:]
        rows = {}
        for line in f:
            cells = line.rstrip("\n").split("\t")
            rows[cells[0]] = np.array([float(x) for x in cells[1:]], dtype=np.float64)
    return files, rows


def read_queries(path):
    lines = [l.rstrip("\n").split("\t") for l in open(path, encoding="utf-8") if l.strip() and not l.startswith("#")]
    header, rows = lines[0], lines[1:]
    d, c = header.index("description"), header.index("class")
    return [(r[d], None if r[c] == "-" else r[c]) for r in rows]


def nearest_rank_median(scores):
    ordered = np.sort(scores)
    return ordered[int(np.floor((len(ordered) - 1) * 0.5 + 0.5))]


def auc(a, u):
    greater = (a[:, None] > u[None, :]).sum()
    ties = (a[:, None] == u[None, :]).sum()
    return (greater + 0.5 * ties) / (len(a) * len(u))


def wilson(k, n, z=1.96):
    p = k / n
    centre = (p + z * z / (2 * n)) / (1 + z * z / n)
    half = z * np.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    return max(0.0, centre - half), min(1.0, centre + half)


def interval(x):
    lo, hi = np.percentile(x, [2.5, 97.5])
    return "[%+.3f, %+.3f]" % (lo, hi)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("dump")
    parser.add_argument("queries")
    parser.add_argument("--resamples", type=int, default=4000)
    parser.add_argument("--seed", type=int, default=113)
    args = parser.parse_args()

    _, rows = read_dump(args.dump)
    queries = read_queries(args.queries)
    raw_top, centred_top, answerable = [], [], []
    for description, cls in queries:
        scores = rows[description]
        raw_top.append(scores.max())
        centred_top.append(scores.max() - nearest_rank_median(scores))
        answerable.append(cls is not None)
    raw_top, centred_top, answerable = np.array(raw_top), np.array(centred_top), np.array(answerable)
    pos, neg = np.where(answerable)[0], np.where(~answerable)[0]

    point_raw, point_centred = auc(raw_top[pos], raw_top[neg]), auc(centred_top[pos], centred_top[neg])
    rng = np.random.default_rng(args.seed)
    raw, centred = np.empty(args.resamples), np.empty(args.resamples)
    for b in range(args.resamples):
        p, n = rng.choice(pos, len(pos)), rng.choice(neg, len(neg))
        raw[b], centred[b] = auc(raw_top[p], raw_top[n]), auc(centred_top[p], centred_top[n])
    diff = centred - raw

    print("%d answerable and %d unanswerable descriptions; %d resamples of the descriptions, seed %d" % (len(pos), len(neg), args.resamples, args.seed))
    print("  AUC, raw cosine          %.3f   95%% interval %s" % (point_raw, interval(raw)))
    print("  AUC, less query median   %.3f   95%% interval %s" % (point_centred, interval(centred)))
    print("  centred minus raw        %+.3f   95%% interval %s   share of resamples where centring is lower: %.1f %%" % (
        point_centred - point_raw, interval(diff), 100.0 * (diff < 0).mean()))
    print("  share of resamples where the raw AUC is below 0.85: %.1f %%   below 0.90: %.1f %%" % (100.0 * (raw < 0.85).mean(), 100.0 * (raw < 0.90).mean()))
    for floor in (0.20, 0.35):
        k = int((raw_top[pos] < floor).sum())
        lo, hi = wilson(k, len(pos))
        print("  answerable descriptions whose best result is under %.2f: %d of %d   Wilson 95%% interval [%.0f %%, %.0f %%]" % (floor, k, len(pos), 100 * lo, 100 * hi))


if __name__ == "__main__":
    main()
