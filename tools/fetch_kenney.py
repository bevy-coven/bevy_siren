#!/usr/bin/env python3
"""Fetch the golden set's second library — Kenney's CC0 game audio — verify every file against this repository's manifest, and write its sidecars.

    python3 tools/fetch_kenney.py <dir>             # the one command: downloads what is missing, verifies every file
    python3 tools/fetch_kenney.py <dir> --record    # maintainers only: derive golden/kenney/labels.csv and manifest.sha256 from the packs

`<dir>` ends up holding the set and nothing that is not in it:

    audio/<pack>/<file>   the labelled files, as shipped: Ogg Vorbis, MP3 and WAV (SHA-256 checked against golden/kenney/manifest.sha256)
    licences/<pack>.txt   each pack's own licence text, as it came in the archive
    labels.csv            file,class      — copied from golden/kenney/labels.csv, in the shape `semantic::golden` reads
    licences.csv          file,licence,source,author — made here; the sidecar `semantic::indexer` reads, so every indexed file carries
                          its licence and the page it came from
    .packs/*.zip          the archives, kept so a second run needs no network (delete freely)

**Where the audio comes from.** Seven packs by Kenney (kenney.nl), published CC0 and hosted on OpenGameArt, whose static file URLs this
fetches. `kenney.nl`'s own download link answers a script with 403, and that is not worked around: the same archives are on OpenGameArt, each
page listing `License(s): CC0` and the author. Each archive is checked against the SHA-256 recorded *in this repository* before a byte of it
is used (ADR 0005's rule: a hash is compared against a value recorded here, never one delivered beside the payload), and then each extracted
file against its own line, so a change in any byte is refused by name.

**What the labels are.** The author's own names, grouped: `footstep00` … `footstep09` are class `footstep`; `laser1` … `laser9` of the Digital pack
and `laserLarge_000` … `laserSmall_004` of the Sci-Fi pack are class `laser`. A class is a family a developer would type a description of (RULES below),
and nothing here is a listener's judgement of what a sound *is*: a label is only as good as the author's file name. Families whose names say nothing
a person would type — `rollover`, `glitch`, `scratch`, `tone`, `pep` — and the two voice-over packs are not in the set at all, and **so are
all 38 `switch` files of the UI pack**, which would be an eighth of the library and one class. That rule was written before the model ever ran on
this set (`docs/loop/runs/2026-11-03-113/plan.md`), on the file names and the size of the family alone.

Licence: **CC0** (https://creativecommons.org/publicdomain/zero/1.0/). Unlike ESC-50 it may be vendored; it is fetched anyway because the audio is
12 MB the repository does not need, and the manifest makes the copy exact.

Needs only the Python standard library and a network connection the first time (the seven archives are 11.9 MB, kept in `.packs/`).
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
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
GOLDEN = os.path.join(HERE, "..", "golden", "kenney")
MANIFEST = os.path.join(GOLDEN, "manifest.sha256")
LABELS = os.path.join(GOLDEN, "labels.csv")
BASE = "https://opengameart.org/sites/default/files/"
LICENCE = "CC0"
AUTHOR = "Kenney (www.kenney.nl)"
AUDIO = (".ogg", ".mp3", ".wav")

# slug: (archive, the pack's page — where its licence and author are listed, the archive member that carries its own licence text)
PACKS = {
    "rpg": ("RPGsounds_Kenney.zip", "https://opengameart.org/content/50-rpg-sound-effects", "license.txt"),
    "uisfx": ("UI_SFX_Set.zip", "https://opengameart.org/content/51-ui-sound-effects-buttons-switches-and-clicks", "readme.txt"),
    "digital": ("Digital_SFX_Set.zip", "https://opengameart.org/content/63-digital-sound-effects-lasers-phasers-space-etc", "readme.txt"),
    "casino": ("kenney_casino-audio.zip", "https://opengameart.org/content/54-casino-sound-effects-cards-dice-chips", "License.txt"),
    "jingles": ("jingleSounds_Kenney.zip", "https://opengameart.org/content/85-short-music-jingles", "license.txt"),
    "interface": ("kenney_interfaceSounds.zip", "https://opengameart.org/content/interface-sounds", "License.txt"),
    "scifi": ("sci-fi_sounds.zip", "https://opengameart.org/content/sci-fi-sounds", "License.txt"),
}

# (pack, a regular expression that must match a file's whole name without its extension, class).
# A file no rule claims is left out of the set; a file two rules claim for different classes is refused.
RULES = [
    # RPG foley (Ogg)
    ("rpg", r"footstep\d+", "footstep"),
    ("rpg", r"door(Open|Close)_\d+", "door"),
    ("rpg", r"book(Close|Flip\d+|Open|Place\d+)", "book"),
    ("rpg", r"cloth\d+", "cloth"),
    ("rpg", r"(drawKnife|knifeSlice)\d*", "knife"),
    ("rpg", r"metalPot\d+", "metal_pot"),
    ("rpg", r"(dropLeather|handleSmallLeather\d*)", "leather"),
    # casino (Ogg): the page says "23 card handling sounds, 19 chip handling and 12 dice handling"
    ("casino", r"cards?-.+", "playing_cards"),
    ("casino", r"chips?-.+", "poker_chips"),
    ("casino", r"(dice|die)-.+", "dice"),
    # short jingles (Ogg): the page says "17 cues each in 8-bit, hit, pizzicato, sax and steel groups"; `HIT` is not a family a person describes
    ("jingles", r"jingles_NES\d+", "chiptune"),
    ("jingles", r"jingles_SAX\d+", "saxophone"),
    ("jingles", r"jingles_STEEL\d+", "steel_drum"),
    ("jingles", r"jingles_PIZZA\d+", "pizzicato"),
    # UI: a click is a click in either pack (WAV and Ogg)
    ("uisfx", r"(click\d+|mouseclick\d+|mouserelease\d+)", "mouse_click"),
    ("interface", r"click_\d+", "mouse_click"),
    ("interface", r"error_\d+", "error_beep"),
    ("interface", r"confirmation_\d+", "confirmation_chime"),
    ("interface", r"scroll_\d+", "scrolling"),
    # digital (MP3): the page says "9 lasers, 15 phasers, 12 power-ups, 6 zaps"
    ("digital", r"laser\d+", "laser"),
    ("digital", r"powerUp\d+", "power_up"),
    ("digital", r"zap.*", "electric_zap"),
    ("digital", r"(phaserUp|phaserDown|phaseJump)\d+", "phaser"),
    # sci-fi (Ogg)
    ("scifi", r"laser(Large|Retro|Small)_\d+", "laser"),
    ("scifi", r"(explosionCrunch|lowFrequency_explosion)_\d+", "explosion"),
    ("scifi", r"forceField_\d+", "force_field"),
    ("scifi", r"door(Open|Close)_\d+", "sci_fi_door"),
    ("scifi", r"(spaceEngine(Large|Low|Small)?|engineCircular)_\d+", "engine"),
    ("scifi", r"thrusterFire_\d+", "thruster"),
    ("scifi", r"computerNoise_\d+", "computer_noise"),
    ("scifi", r"impactMetal_\d+", "metal_impact"),
]

# The size every class must come out at. A pack that changed under the same name is refused rather than silently relabelled.
SIZES = {
    "footstep": 10, "door": 6, "book": 8, "cloth": 4, "knife": 5, "metal_pot": 3, "leather": 3,
    "playing_cards": 23, "poker_chips": 19, "dice": 12,
    "chiptune": 17, "saxophone": 17, "steel_drum": 17, "pizzicato": 17,
    "mouse_click": 12, "error_beep": 8, "confirmation_chime": 4, "scrolling": 5,
    "laser": 24, "power_up": 12, "electric_zap": 6, "phaser": 15, "explosion": 7, "force_field": 5,
    "sci_fi_door": 6, "engine": 24, "thruster": 5, "computer_noise": 4, "metal_impact": 5,
}


def fail(message, code=1):
    print("fetch_kenney: " + message, file=sys.stderr)
    sys.exit(code)


def fnv1a64(data):
    """The same 64-bit FNV-1a `library::fnv1a` computes, so this script and `examples/goldenbench` print one id for one set."""
    h = 0xCBF29CE484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def download(url, attempts=4):
    last = None
    for attempt in range(attempts):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "bevy_siren-fetch_kenney"})
            with urllib.request.urlopen(request, timeout=120) as response:
                return response.read()
        except OSError as e:  # URLError and HTTPError are OSErrors; a 429 or a reset is worth another try
            last = e
            time.sleep(1.5 ** attempt)
    fail("cannot download %s: %s" % (url, last), 2)


def read_manifest():
    """`(version, {pack: archive sha256}, {library path: sha256})`; the pinned version is the comment line `# version: <v>`."""
    if not os.path.isfile(MANIFEST):
        fail("%s does not exist; run with --record once to record the set" % MANIFEST)
    version, packs, entries = None, {}, {}
    for line in open(MANIFEST, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("# version:"):
            version = line.split(":", 1)[1].strip()
        elif line.startswith("# pack:"):
            parts = line.split(":", 1)[1].split()
            if len(parts) != 3 or len(parts[1]) != 64 or parts[0] not in PACKS or PACKS[parts[0]][0] != parts[2]:
                fail("%s: a `# pack:` line that is not `<pack> <sha256> <archive>` for a pack this script fetches: %r" % (MANIFEST, line))
            packs[parts[0]] = parts[1]
        if not line or line.startswith("#"):
            continue
        digest, _, path = line.partition("  ")
        if len(digest) != 64 or not path:
            fail("%s: a line that is not `<sha256>  <path>`: %r" % (MANIFEST, line))
        entries[path] = digest
    if set(packs) != set(PACKS):
        fail("the manifest pins packs %s and this script fetches %s" % (sorted(packs), sorted(PACKS)))
    if not version:
        fail("the manifest names no `# version:`")
    return version, packs, entries


def read_labels():
    """`{library path: class}` from the checked-in labels."""
    if not os.path.isfile(LABELS):
        fail("%s does not exist; run with --record once to record the set" % LABELS)
    with open(LABELS, newline="", encoding="utf-8") as f:
        return {row["file"]: row["class"] for row in csv.DictReader(f)}


def open_pack(out, slug, expected):
    """The pack's archive: the cached copy when it is the pinned one, else downloaded and checked **before** it is written."""
    name = PACKS[slug][0]
    cache = os.path.join(out, ".packs", name)
    if os.path.isfile(cache) and (expected is None or sha256_of(cache) == expected):
        return zipfile.ZipFile(cache)
    url = BASE + urllib.parse.quote(name)
    data = download(url)
    if expected is not None and sha256_bytes(data) != expected:
        fail("%s has different bytes from the pinned archive (SHA-256 %s, not %s); not written" % (url, sha256_bytes(data), expected))
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    with open(cache + ".part", "wb") as f:
        f.write(data)
    os.replace(cache + ".part", cache)
    return zipfile.ZipFile(cache)


def members_by_name(archive):
    """`{file name: member path}`. The library names files by name, so two members with one name are refused."""
    found = {}
    for member in archive.namelist():
        if member.endswith("/"):
            continue
        name = os.path.basename(member)
        if name in found:
            fail("%s holds two members named %s (%s and %s)" % (archive.filename, name, found[name], member))
        found[name] = member
    return found


def classify(slug, stem):
    classes = {c for (pack, pattern, c) in RULES if pack == slug and re.fullmatch(pattern, stem)}
    if len(classes) > 1:
        fail("%s/%s is claimed for %d classes: %s" % (slug, stem, len(classes), sorted(classes)))
    return next(iter(classes), None)


def write_file(out, path, data):
    target = os.path.join(out, path)
    os.makedirs(os.path.dirname(target), exist_ok=True)
    with open(target + ".part", "wb") as f:
        f.write(data)
    os.replace(target + ".part", target)


def stray_files(out, entries):
    """Files in audio/ that are not part of the set: they would join every ranking and change every number."""
    root = os.path.join(out, "audio")
    found = []
    for folder, _, files in os.walk(root):
        for name in files:
            path = os.path.relpath(os.path.join(folder, name), out).replace(os.sep, "/")
            if path not in entries:
                found.append(path)
    return sorted(found)


def record(out):
    """Derive the labels and the manifest from the packs as they are now. The one moment either is made, so nothing here is verified."""
    entries, labels, left_out, packs = {}, {}, {}, {}
    for slug in PACKS:
        archive = open_pack(out, slug, None)
        packs[slug] = sha256_of(archive.filename)
        names = members_by_name(archive)
        left = 0
        for name in sorted(names):
            stem, extension = os.path.splitext(name)
            if extension.lower() not in AUDIO:
                continue
            cls = classify(slug, stem)
            if cls is None:
                left += 1
                continue
            path = "audio/%s/%s" % (slug, name)
            data = archive.read(names[name])
            write_file(out, path, data)
            entries[path], labels[path] = sha256_bytes(data), cls
        left_out[slug] = left
        licence = PACKS[slug][2]
        if licence not in names:
            fail("%s has no member %s to carry its licence text" % (archive.filename, licence))
        text = archive.read(names[licence])
        write_file(out, "licences/%s.txt" % slug, text)
        entries["licences/%s.txt" % slug] = sha256_bytes(text)

    sizes = {}
    for cls in labels.values():
        sizes[cls] = sizes.get(cls, 0) + 1
    if sizes != SIZES:
        wrong = {c: (sizes.get(c, 0), SIZES.get(c, 0)) for c in sorted(set(sizes) | set(SIZES)) if sizes.get(c, 0) != SIZES.get(c, 0)}
        fail("the rules gave different class sizes from SIZES — a pack changed, or a rule is wrong (class: got, expected): %s" % wrong)
    by_hash = {}
    for path, digest in entries.items():
        if path.startswith("audio/"):
            by_hash.setdefault(digest, []).append(path)
    shared = {d: p for d, p in by_hash.items() if len(p) > 1}
    if shared:
        fail("these files are the same bytes under two names, so the library would hold one asset for two rows: %s" % list(shared.values()))

    os.makedirs(GOLDEN, exist_ok=True)
    with open(LABELS, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(["file", "class"])
        writer.writerows(sorted(labels.items()))
    with open(MANIFEST, "w", encoding="utf-8") as f:
        f.write("# Kenney's CC0 game audio (https://kenney.nl), as hosted on OpenGameArt — the files this golden set is made of.\n")
        f.write("# version: v1\n")
        f.write("# One `# pack:` line per archive: the pack, the SHA-256 of the archive, its file name (fetched from %s).\n" % BASE)
        for slug in PACKS:
            f.write("# pack: %s %s %s\n" % (slug, packs[slug], PACKS[slug][0]))
        f.write("# One line per file: SHA-256, two spaces, the path under the library directory. `tools/fetch_kenney.py` checks every one,\n")
        f.write("# so a download that differs in any byte is refused by name. Regenerate only on purpose: a changed line is a changed set,\n")
        f.write("# and every recorded number is about the old one. `labels.csv` beside this file says which class each audio file is in.\n")
        for path in sorted(entries):
            f.write("%s  %s\n" % (entries[path], path))
    print("recorded %d audio files in %d classes (left out of the set, by pack: %s)" % (
        len(labels), len(sizes), ", ".join("%s %d" % kv for kv in left_out.items())))
    print("wrote %s and %s" % (os.path.relpath(LABELS), os.path.relpath(MANIFEST)))


def write_licences(out, audio):
    """`licences.csv`: a row for **every** file — a licence line per asset is the point."""
    with open(os.path.join(out, "licences.csv"), "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(["file", "licence", "source", "author"])
        for path in sorted(audio):
            writer.writerow([path, LICENCE, PACKS[path.split("/")[1]][1], AUTHOR])


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("dir", help="the library directory to fill")
    parser.add_argument("--record", action="store_true", help="derive the labels and the manifest from the packs (maintainers only)")
    args = parser.parse_args()
    out = os.path.abspath(args.dir)
    os.makedirs(out, exist_ok=True)

    if args.record:
        record(out)

    version, packs, entries = read_manifest()
    labels = read_labels()
    audio = {path for path in entries if path.startswith("audio/")}
    if set(labels) != audio:
        only_labels, only_manifest = sorted(set(labels) - audio), sorted(audio - set(labels))
        fail("labels.csv and the manifest list different files (only in the labels: %s; only in the manifest: %s)" % (only_labels[:3], only_manifest[:3]))
    stray = stray_files(out, entries)
    if stray:
        for path in stray[:10]:
            print("  " + path, file=sys.stderr)
        fail("%d files in audio/ are not part of the set (they would change every number); remove them" % len(stray))

    # What is missing or wrong is extracted; what is already right is left alone, so a second run is quick.
    wanted = {}
    for path, digest in entries.items():
        full = os.path.join(out, path)
        if not (os.path.isfile(full) and sha256_of(full) == digest):
            wanted.setdefault(os.path.splitext(os.path.basename(path))[0] if path.startswith("licences/") else path.split("/")[1], {})[path] = digest
    problems = []
    for slug, files in wanted.items():
        archive = open_pack(out, slug, packs[slug])
        names = members_by_name(archive)
        for path, digest in sorted(files.items()):
            name = PACKS[slug][2] if path.startswith("licences/") else os.path.basename(path)
            if name not in names:
                problems.append("not in %s: %s" % (PACKS[slug][0], path))
                continue
            data = archive.read(names[name])
            if sha256_bytes(data) != digest:
                problems.append("different bytes from the pinned archive (not written): " + path)
                continue
            write_file(out, path, data)
    if problems:
        for problem in problems[:10]:
            print("  " + problem, file=sys.stderr)
        fail("%d files disagree with the manifest (the first %d are listed)" % (len(problems), min(10, len(problems))))

    with open(LABELS, "rb") as f:
        write_file(out, "labels.csv", f.read())
    write_licences(out, audio)
    identity = "%016x" % fnv1a64(open(MANIFEST, "rb").read())
    print("kenney @ %s: %d files verified against the manifest (set id %s)" % (version, len(audio), identity))
    print("  classes: %d; licence: %s on every file, from %d packs" % (len(set(labels.values())), LICENCE, len(packs)))
    print("  wrote %s and %s" % (os.path.join(out, "labels.csv"), os.path.join(out, "licences.csv")))


if __name__ == "__main__":
    main()
