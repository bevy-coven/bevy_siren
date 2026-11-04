"""Hubness, and what normalising for it does — an exploration on two libraries, in Python only. Nothing here is in the crate.

    python3 normalisations.py <library-dir> <dump.tsv> <queries.tsv>

`dump.tsv` is `goldenbench --dump`. Every similarity here is the crate's own; the normalisations are applied to that matrix.

**Why.** The second library found that a few gallery items are the nearest neighbour of many unrelated descriptions: a hub. Querybank
Normalisation (Bogolin et al., CVPR 2022, DOI 10.1109/cvpr52688.2022.00513) names the problem — *"a small number of gallery embeddings form
the nearest neighbours of many queries"* — and re-normalises each query's similarities for it. This is the simplest version of its idea, not
its method: each file's mean similarity to the **bank of the set's own descriptions with the query being asked left out** is subtracted
(`gallery-centred`), optionally divided by the bank's standard deviation for that file (`gallery z-score`). `query-centred` is the crate's
`eval::centred`: the query's own median, which the second library showed does not help.

Hubness itself is read as the **skewness of N3**, how many descriptions have each file in their top three (Radovanovic, Nanopoulos & Ivanovic's
measure, in the form QB-Norm's paper uses), and as who takes the top result.
"""
import csv
import os
import sys

import numpy as np
from scipy.stats import skew


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


def auc(positive, negative):
    return sum(1.0 if p > n else 0.5 if p == n else 0.0 for p in positive for n in negative) / (len(positive) * len(negative))


def evaluate(matrix, truth, queries):
    """`matrix` is (queries, files). hit@1/3/10, MRR, the right answers' median score, and the separation of the top score."""
    hit = {1: [], 3: [], 10: []}
    rr, right_scores = [], []
    for i, (description, cls) in enumerate(queries):
        if cls is None:
            continue
        order = np.lexsort((np.arange(matrix.shape[1]), -matrix[i]))
        right = truth[order] == cls
        for k in hit:
            hit[k].append(right[:k].any())
        first = np.flatnonzero(right)
        rr.append(1.0 / (first[0] + 1) if len(first) else 0.0)
        right_scores.extend(matrix[i][truth == cls])
    tops = {c is None: [] for _, c in queries}
    answerable = [matrix[i].max() for i, (_, c) in enumerate(queries) if c is not None]
    unanswerable = [matrix[i].max() for i, (_, c) in enumerate(queries) if c is None]
    return {
        "hit_1": np.mean(hit[1]), "hit_3": np.mean(hit[3]), "hit_10": np.mean(hit[10]), "mrr": np.mean(rr),
        "right_median": float(np.median(right_scores)), "auc": auc(answerable, unanswerable),
        "weakest": min(answerable), "strongest": max(unanswerable),
    }


def main():
    directory, dump, queries_path = sys.argv[1:4]
    files, rows = read_dump(dump)
    labels = {}
    with open(os.path.join(directory, "labels.csv"), newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            labels[row["file"]] = row["class"]
    truth = np.array([labels[f] for f in files])
    queries = read_queries(queries_path)
    S = np.stack([rows[d] for d, _ in queries])  # (queries, files)
    n_queries = S.shape[0]

    # --- hubness
    top3 = np.argsort(-S, axis=1)[:, :3]
    n3 = np.bincount(top3.ravel(), minlength=S.shape[1])
    top1 = S.argmax(axis=1)
    top1_files, top1_counts = np.unique(top1, return_counts=True)
    order = np.argsort(-top1_counts)
    classes_first = {}
    for j in top1:
        classes_first[truth[j]] = classes_first.get(truth[j], 0) + 1
    print("%s: %d files, %d descriptions (%d answerable, %d not)" % (os.path.basename(os.path.abspath(directory)), len(files), n_queries,
          sum(c is not None for _, c in queries), sum(c is None for _, c in queries)))
    print("hubness: skewness of N3 (how often each file is in a description's top three) %.2f; %d of %d files are in no top three; the most-listed file is in %d"
          % (skew(n3), int((n3 == 0).sum()), len(files), n3.max()))
    print("         the top result is one of only %d distinct files; the most common is %s, first for %d of %d descriptions"
          % (len(top1_files), files[top1_files[order[0]]], top1_counts[order[0]], n_queries))
    ranked = sorted(classes_first.items(), key=lambda kv: -kv[1])[:3]
    print("         by class, the top result is: %s" % ", ".join("%s %d" % kv for kv in ranked))

    # --- the normalisations
    total = S.sum(axis=0)
    loo_mean = (total[None, :] - S) / (n_queries - 1)                                  # each file's mean over the other descriptions
    loo_sq = ((S ** 2).sum(axis=0)[None, :] - S ** 2) / (n_queries - 1)
    loo_std = np.sqrt(np.maximum(loo_sq - loo_mean ** 2, 1e-12))
    variants = {
        "the cosine itself": S,
        "query-centred (eval::centred)": S - np.median(S, axis=1, keepdims=True),
        "gallery-centred (QB-Norm's idea)": S - loo_mean,
        "gallery z-score": (S - loo_mean) / loo_std,
    }
    print("\n%-34s %7s %7s %8s %6s %14s %5s %9s %11s" % ("", "hit@1", "hit@3", "hit@10", "MRR", "right median", "AUC", "weakest", "strongest"))
    for name, matrix in variants.items():
        m = evaluate(matrix, truth, queries)
        print("%-34s %6.1f%% %6.1f%% %7.1f%% %6.3f %+14.3f %5.3f %+9.3f %+11.3f" % (
            name, 100 * m["hit_1"], 100 * m["hit_3"], 100 * m["hit_10"], m["mrr"], m["right_median"], m["auc"], m["weakest"], m["strongest"]))
    print("(AUC: an answerable description's top score above an unanswerable one's; weakest/strongest: lowest answerable top, highest unanswerable top)")


if __name__ == "__main__":
    main()
