#!/usr/bin/env python3
"""Fetch the golden set's audio (ESC-50), verify every file against the manifest in this repository, and write its sidecars.

    python3 tools/fetch_esc50.py <dir>                    # the one command: downloads what is missing, verifies every file
    python3 tools/fetch_esc50.py <dir> --write-manifest   # maintainers only: record what the pinned commit contains

`<dir>` ends up holding the set and nothing that is not in it:

    audio/*.wav       2000 clips, 5 s, 44.1 kHz mono      (SHA-256 checked against golden/esc50/manifest.sha256)
    meta/esc50.csv    the dataset's own labels
    LICENSE           the dataset's licence, with every clip's author and licence
    labels.csv        file,class      — made here from meta/esc50.csv, in the shape `semantic::golden` reads
    licences.csv      file,licence,source,author — made here from LICENSE; it is the sidecar `semantic::indexer` reads, so
                      every indexed clip carries its licence and where it came from

**Files, not an archive.** Each file is fetched from `raw.githubusercontent.com` at the pinned commit, 16 at a time, and checked
against the manifest *as it arrives*: a file that disagrees is refused by name and never written. (The one-archive route was tried
first: GitHub streamed it at ~100 KB/s from here, which is two hours for 600 MB, where the files take about a minute.) A second run
downloads only what is missing or wrong, so an interrupted run resumes.

**Why a manifest and not a pinned archive hash.** GitHub builds an archive on request and its bytes are not promised stable, so
a hash of an archive verifies nothing for long. The commit is immutable and so are the files in it: the manifest pins each file.
ADR 0005's rule applies — a hash is compared against a value recorded *in this repository*, never against one delivered beside
the payload.

**Licence.** The dataset as a whole is CC BY-NC 3.0; the ESC-10 subset is CC BY 3.0; each clip keeps its own Freesound licence
(LICENSE lists them). It is fetched for evaluation and **never vendored or packaged**: `Cargo.toml`'s `include` does not name it.

Needs only the Python standard library and a network connection the first time (about 880 MB).
"""
import argparse
import csv
import hashlib
import os
import re
import sys
import time
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor

REPO = "karolpiczak/ESC-50"
COMMIT = "33c8ce9eb2cf0b1c2f8bcf322eb349b6be34dbb6"  # master on 2023-10-13; the dataset has not changed since 2020
RAW = "https://raw.githubusercontent.com/%s/%s/" % (REPO, COMMIT)
WORKERS = 16
HERE = os.path.dirname(os.path.abspath(__file__))
GOLDEN = os.path.join(HERE, "..", "golden", "esc50")
MANIFEST = os.path.join(GOLDEN, "manifest.sha256")
QUERIES = os.path.join(GOLDEN, "queries.tsv")
EXTRA = ("meta/esc50.csv", "LICENSE")  # what is kept besides audio/*.wav
# `{fold}-{freesound id}-{take}-{target}.wav`, as the dataset's README names it.
CLIP = re.compile(r"^audio/(\d+-\d+-[A-Z])-(\d+)\.wav$")
# `- [1-100032-A.ogg]: clip derived from rose_bark.wav (http://…/sounds/100032/) by nfrae [CC0]`
# The title is lazy and the space before the URL optional, because one line is `[DrinkingCan[Norberd]](http://…)`.
LICENCE_LINE = re.compile(
    r"^- \[(?P<clip>[^\]]+?)\.ogg\]:\s+clip derived from (?P<title>.*?)\s*\((?P<url>https?://[^\s)]+)\) by (?P<author>.+?) \[(?P<licence>[^\]]+)\]\s*$"
)
# **The dataset's own typos, named rather than papered over.** LICENSE lists `5-141683-Ad.ogg`; the clip is `5-141683-A` (the line's
# own URL is Freesound sound 141683). Every row is also checked against its clip's Freesound id below, so a *wrong* correction
# cannot hide here: it would be a mismatch.
LICENSE_TYPOS = {"5-141683-Ad": "5-141683-A"}


def fail(message, code=1):
    print("fetch_esc50: " + message, file=sys.stderr)
    sys.exit(code)


def fnv1a64(data):
    """The same 64-bit FNV-1a `library::fnv1a` computes, so this script and `examples/goldenbench` print one id for one set."""
    h = 0xCBF29CE484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def read_manifest():
    """`{relative path: sha256}`; the pinned commit is the comment line `# commit: <sha>`."""
    if not os.path.isfile(MANIFEST):
        fail("%s does not exist; run with --write-manifest once to record the set" % MANIFEST)
    entries, commit = {}, None
    for line in open(MANIFEST, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("# commit:"):
            commit = line.split(":", 1)[1].strip()
        if not line or line.startswith("#"):
            continue
        digest, _, path = line.partition("  ")
        if len(digest) != 64 or not path:
            fail("%s: a line that is not `<sha256>  <path>`: %r" % (MANIFEST, line))
        entries[path] = digest
    if commit != COMMIT:
        fail("the manifest records commit %s and this script fetches %s" % (commit, COMMIT))
    return entries


def present_and_right(out, path, digest):
    full = os.path.join(out, path)
    return os.path.isfile(full) and sha256_of(full) == digest


def fetch(relative, attempts=4):
    url = RAW + urllib.parse.quote(relative)
    last = None
    for attempt in range(attempts):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "bevy_siren-fetch_esc50"})
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except OSError as e:  # URLError and HTTPError are OSErrors; a 429 or a reset is worth another try
            last = e
            time.sleep(1.5 ** attempt)
    fail("cannot download %s: %s" % (url, last), 2)


def pull(out, relative, expected):
    """Download one file; with `expected`, refuse it by name unless its SHA-256 matches. Returns a problem or None."""
    data = fetch(relative)
    if expected is not None and hashlib.sha256(data).hexdigest() != expected:
        return "different bytes from the pinned commit (not written): " + relative
    target = os.path.join(out, relative)
    os.makedirs(os.path.dirname(target), exist_ok=True)
    with open(target + ".part", "wb") as f:
        f.write(data)
    os.replace(target + ".part", target)
    return None


def pull_all(out, wanted):
    """`wanted` is `{path: expected sha256 or None}`. Fetched in parallel; returns the list of problems."""
    problems, done = [], 0
    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        for problem in pool.map(lambda item: pull(out, item[0], item[1]), sorted(wanted.items())):
            done += 1
            if problem:
                problems.append(problem)
            if done % 250 == 0 or done == len(wanted):
                print("  %d / %d files" % (done, len(wanted)), file=sys.stderr)
    return problems


def stray_clips(out, entries):
    """`.wav` files in audio/ that are not part of the set: they would join every ranking and change every number."""
    audio = os.path.join(out, "audio")
    if not os.path.isdir(audio):
        return []
    return ["audio/" + n for n in sorted(os.listdir(audio)) if n.endswith(".wav") and "audio/" + n not in entries]


def write_manifest(out):
    paths = sorted(
        ["audio/" + n for n in os.listdir(os.path.join(out, "audio")) if n.endswith(".wav")] + list(EXTRA)
    )
    os.makedirs(GOLDEN, exist_ok=True)
    with open(MANIFEST, "w", encoding="utf-8") as f:
        f.write("# ESC-50 (https://github.com/%s), the files this golden set is made of.\n" % REPO)
        f.write("# commit: %s\n" % COMMIT)
        f.write("# One line per file: SHA-256, two spaces, the path under the library directory. `tools/fetch_esc50.py`\n")
        f.write("# checks every one, so a download that differs in any byte is refused by name. Regenerate only on purpose:\n")
        f.write("# a changed line is a changed set, and every recorded number is about the old one.\n")
        for path in paths:
            f.write("%s  %s\n" % (sha256_of(os.path.join(out, path)), path))
    print("recorded %d files in %s" % (len(paths), os.path.relpath(MANIFEST)))


def class_names():
    """`{target: class}` from the checked-in queries, so the labels can be cross-checked against the dataset's own."""
    names = {}
    if os.path.isfile(QUERIES):
        for line in open(QUERIES, encoding="utf-8"):
            if line.startswith("#") or not line.strip():
                continue
            cells = line.rstrip("\n").split("\t")
            if len(cells) >= 2 and cells[0].isdigit():
                names[int(cells[0])] = cells[1]
    return names


def write_labels(out):
    """`labels.csv` (file,class) from the dataset's own CSV — and the proof that the two spellings of a label agree."""
    names = class_names()
    rows, seen = [], {}
    with open(os.path.join(out, "meta", "esc50.csv"), newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            target, category = int(row["target"]), row["category"]
            clip = CLIP.match("audio/" + row["filename"])
            if not clip or int(clip.group(2)) != target:
                fail("meta/esc50.csv: %s is labelled %d but its name says %s" % (row["filename"], target, clip and clip.group(2)))
            if seen.setdefault(target, category) != category:
                fail("meta/esc50.csv: target %d is both %r and %r" % (target, seen[target], category))
            if names and names.get(target) not in (None, category):
                fail("queries.tsv calls target %d %r and the dataset calls it %r" % (target, names[target], category))
            rows.append(("audio/" + row["filename"], category))
    with open(os.path.join(out, "labels.csv"), "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["file", "class"])
        writer.writerows(sorted(rows))
    return rows


def clean(value):
    # The crate's CSV reader splits on commas outside double quotes and has no escape for a quote inside one.
    return value.replace('"', "'").strip()


def write_licences(out, clips):
    """`licences.csv` from LICENSE: a row for **every** clip, or this fails — a licence line per asset is the point."""
    by_prefix = {}
    with open(os.path.join(out, "LICENSE"), encoding="utf-8") as f:
        for line in f:
            m = LICENCE_LINE.match(line.rstrip("\n"))
            if m:
                by_prefix[LICENSE_TYPOS.get(m.group("clip"), m.group("clip"))] = m
    rows, absent = [], []
    for path in sorted(clips):
        clip = CLIP.match(path)
        m = by_prefix.get(clip.group(1)) if clip else None
        if m is None:
            absent.append(path)
            continue
        sound = re.search(r"/sounds/(\d+)/?$", m.group("url"))
        if not sound or sound.group(1) != clip.group(1).split("-")[1]:
            fail("LICENSE says %s was derived from %s, which is not Freesound sound %s" % (path, m.group("url"), clip.group(1).split("-")[1]))
        rows.append((path, clean(m.group("licence")), clean(m.group("url")), clean(m.group("author"))))
    if absent:
        fail("LICENSE has no line for %d clips, the first being %s" % (len(absent), absent[0]))
    with open(os.path.join(out, "licences.csv"), "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["file", "licence", "source", "author"])
        writer.writerows(rows)
    counts = {}
    for row in rows:
        counts[row[1]] = counts.get(row[1], 0) + 1
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("dir", help="the library directory to fill")
    parser.add_argument("--write-manifest", action="store_true", help="record the pinned commit's files (maintainers only)")
    args = parser.parse_args()
    out = os.path.abspath(args.dir)
    os.makedirs(out, exist_ok=True)

    if args.write_manifest:
        # Unverified by definition: this is the one moment the manifest is made. Labels first, because they name the clips.
        problems = pull_all(out, {path: None for path in EXTRA})
        with open(os.path.join(out, "meta", "esc50.csv"), newline="", encoding="utf-8") as f:
            clips = ["audio/" + row["filename"] for row in csv.DictReader(f)]
        problems += pull_all(out, {path: None for path in clips})
        if problems:
            fail("%d downloads failed: %s" % (len(problems), "; ".join(problems[:3])))
        write_manifest(out)

    entries = read_manifest()
    stray = stray_clips(out, entries)
    if stray:
        for path in stray[:10]:
            print("  " + path, file=sys.stderr)
        fail("%d files in audio/ are not part of the set (they would change every number); remove them" % len(stray))
    # What is missing or wrong is fetched; what is already right is left alone, so a second run is quick and a dead one resumes.
    wanted = {path: digest for path, digest in entries.items() if not present_and_right(out, path, digest)}
    if wanted:
        print("fetching %d of %d files from %s" % (len(wanted), len(entries), RAW), file=sys.stderr)
        problems = pull_all(out, wanted)
        if problems:
            for p in problems[:10]:
                print("  " + p, file=sys.stderr)
            fail("%d files disagree with the manifest (the first %d are listed)" % (len(problems), min(10, len(problems))))

    clips = [p for p in entries if p.startswith("audio/")]
    labelled = write_labels(out)
    if sorted(r[0] for r in labelled) != sorted(clips):
        fail("meta/esc50.csv labels a different set of files from the manifest")
    counts = write_licences(out, clips)
    identity = "%016x" % fnv1a64(open(MANIFEST, "rb").read())
    print("esc50 @ %s: %d clips verified against the manifest (set id %s)" % (COMMIT[:8], len(clips), identity))
    print("  classes: %d; licences: %s" % (len({c for _, c in labelled}), ", ".join("%s %d" % kv for kv in sorted(counts.items()))))
    print("  wrote %s and %s" % (os.path.join(out, "labels.csv"), os.path.join(out, "licences.csv")))


if __name__ == "__main__":
    main()
