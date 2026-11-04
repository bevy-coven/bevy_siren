"""How far does re-encoding the same 2 000 clips move what the crate computes?

`goldenbench --dump` writes, for every description (and every zero-shot class prompt), the cosine against every clip. This compares the dump of
the WAV library with the dump of the same clips as FLAC and as MP3, clip by clip and description by description, joined on the file's stem.

    python3 format_cost.py <wav.tsv> <other.tsv> [<other.tsv> ...]
"""
import csv
import os
import sys

import numpy as np


def load(path):
    with open(path, newline="") as f:
        rows = list(csv.reader(f, delimiter="\t"))
    stems = [os.path.splitext(name)[0] for name in rows[0][1:]]
    return stems, {r[0]: np.array([float(x) for x in r[1:]]) for r in rows[1:]}


def main():
    base_stems, base = load(sys.argv[1])
    order = {s: i for i, s in enumerate(base_stems)}
    print("%-22s %9s %9s %9s %9s %9s %10s" % ("library vs WAV", "rows", "pairs", "mean |d|", "median", "p99", "max |d|"))
    for path in sys.argv[2:]:
        stems, other = load(path)
        if set(stems) != set(base_stems):
            sys.exit("%s is not the same set of clips" % path)
        # re-order the columns to the WAV library's, because the columns are in hash order and the hashes differ
        perm = np.array([stems.index(s) for s in base_stems])
        diffs = []
        for row, values in base.items():
            if row not in other:
                sys.exit("row %r missing from %s" % (row, path))
            diffs.append(np.abs(other[row][perm] - values))
        d = np.concatenate(diffs)
        name = os.path.basename(path).replace("dump-", "").replace(".tsv", "")
        print("%-22s %9d %9d %9.1e %9.1e %9.1e %10.1e" % (name, len(base), d.size, d.mean(), np.median(d), np.percentile(d, 99), d.max()))


if __name__ == "__main__":
    main()
