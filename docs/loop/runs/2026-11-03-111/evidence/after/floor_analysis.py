"""Can an absolute floor, or a margin, tell a query the library can answer from one it cannot?

    python3 floor_analysis.py <dump.tsv> [queries.tsv]   # the file `goldenbench --dump` wrote, and the queries it was asked

For each of the 50 descriptions with right answers and the 16 that should find nothing, this takes the top-ranked clip's score
(`s1`) and asks how well three things separate the two groups, as the probability that a random answerable query scores higher than
a random unanswerable one (AUC; 0.5 is a coin, 1.0 is perfect):

  - `s1`            the best clip's cosine — what an absolute floor reads
  - `s1 - s10`      how far the best clip stands above the tenth — a margin
  - `s1 - median`   how far it stands above the typical clip in the library

and then, for the floors the report tabulates, how many of each group a floor answers.

The negatives name their confound in the checked-in rationale (a gunshot finds fireworks), so this is a *hard* test of the floor:
it asks for silence where a plausible neighbour exists. It is 66 queries over one library of environmental recordings.
"""
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
QUERIES = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, *[".."] * 6, "golden", "esc50", "queries.tsv")


def auc(positive, negative):
    wins = sum((p > n) + 0.5 * (p == n) for p in positive for n in negative)
    return wins / (len(positive) * len(negative))


def main():
    with open(sys.argv[1], encoding="utf-8") as f:
        f.readline()
        rows = {}
        for line in f:
            cells = line.rstrip("\n").split("\t")
            rows[cells[0]] = np.array([float(x) for x in cells[1:]])
    answerable, silent = [], []
    for line in open(QUERIES, encoding="utf-8"):
        if line.startswith("#") or not line.strip() or line.startswith("target"):
            continue
        cells = line.rstrip("\n").split("\t")
        (silent if cells[1] == "-" else answerable).append(cells[2])

    def features(description):
        s = np.sort(rows[description])[::-1]
        return {"s1": s[0], "s1 - s10": s[0] - s[9], "s1 - median": s[0] - np.median(s)}

    a = [features(d) for d in answerable]
    n = [features(d) for d in silent]
    print("answerable queries: %d, unanswerable: %d\n" % (len(a), len(n)))
    print("%-14s %8s %8s   %8s %8s   %s" % ("", "answ min", "answ med", "none med", "none max", "AUC (answerable above unanswerable)"))
    for name in ("s1", "s1 - s10", "s1 - median"):
        pa, pn = [f[name] for f in a], [f[name] for f in n]
        print("%-14s %+8.3f %+8.3f   %+8.3f %+8.3f   %.3f" % (name, min(pa), float(np.median(pa)), float(np.median(pn)), max(pn), auc(pa, pn)))

    print("\nan absolute floor on s1 — how many of each group still get an answer:")
    print("%6s %12s %14s" % ("floor", "answerable", "unanswerable"))
    for floor in (0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60):
        print("%6.2f %9d/%d %11d/%d" % (floor, sum(f["s1"] >= floor for f in a), len(a), sum(f["s1"] >= floor for f in n), len(n)))
    best = max(((sum(f["s1"] >= t for f in a) - sum(f["s1"] >= t for f in n)), t) for t in np.arange(0.2, 0.7, 0.005))
    print("\nthe floor that maximises (answerable answered - unanswerable answered): %.3f (net %d)" % (best[1], best[0]))


if __name__ == "__main__":
    main()
