#!/usr/bin/env python3
"""**The tour as something a person can play from the internet**: a 32-second MP4, 720p, whose picture names what each
moment demonstrates and whose sound is the crate's own decisions.

    CARGO_INCREMENTAL=0 cargo run --release --example demo_render -- /tmp/demo
    python3 tools/make_demo_video.py /tmp/demo docs/media/demo/tour.mp4

Why a video and not an `<audio>` tag: **GitHub strips `<audio>` and `<video>` from a README**, measured with GitHub's own
`POST /markdown` renderer — both come back as an empty `<p>`, and `![x](a.mp3)` becomes a broken `<img>`. A video uploaded
through GitHub's attachment flow (`user-attachments`) *is* rendered with a player, and a video track can say what is being
demoed. That is the whole design: the picture carries the explanation, the audio carries the evidence.

The picture is drawn here, frame by frame, from `scenes.txt` — the run's own report — so the numbers on screen are the
numbers in the file, not a second set typed into a script.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720
FPS = 30
BARS = 96          # a level meter per bus, recomputed per frame from bus-gain.csv
INK = (232, 236, 244)
MUTED = (138, 148, 168)
BG = (13, 16, 21)
PANEL = (20, 24, 32)
SCENE_INK = (255, 255, 255)
BUS_COLOUR = {
    "Music": (110, 170, 255),
    "Effects": (250, 180, 90),
    "Ambience": (120, 220, 160),
    "Dialogue": (240, 130, 160),
}


def font(size, bold=False):
    for name in (("Menlo-Bold" if bold else "Menlo"), ("SFNSMono",), ("Helvetica",)):
        for ext in (".ttc", ".ttf"):
            path = f"/System/Library/Fonts/{name}{ext}"
            if os.path.exists(path):
                try:
                    return ImageFont.truetype(path, size)
                except OSError:
                    continue
    return ImageFont.load_default()


def parse_report(path):
    """The scenes, their lengths and their numbers, from `demo_render`'s own report."""
    scenes, current = [], None
    for line in path.read_text().splitlines():
        if line.startswith("== "):
            head = line[3:]
            title, _, length = head.partition("   ")
            current = {"title": title.strip(), "len": float(length.split()[0]), "peak": "", "rms": ""}
            scenes.append(current)
        elif current is not None and "peak " in line:
            current["peak"] = line.split("peak ")[1].split()[0]
            current["rms"] = line.split("rms ")[1].split()[0]
    # The last row is the whole tour, which is a label and not a scene.
    return [s for s in scenes if not s["title"].startswith("the whole tour")]


def parse_bus(path):
    """`bus-gain.csv` → `[(t, bus, db)]`, the crate's own numbers."""
    rows = []
    for line in path.read_text().splitlines()[1:]:
        parts = line.split(",")
        if len(parts) == 3:
            rows.append((float(parts[0]), parts[1], float(parts[2])))
    by_bus = {}
    for t, bus, db in rows:
        by_bus.setdefault(bus, []).append((t, db))
    return by_bus


def bus_db_at(history, t):
    """The gain in force at `t`: the last write at or before it, which is what the mixer holds between frames."""
    lo, hi = 0, len(history) - 1
    best = history[0][1] if history else 0.0
    while lo <= hi:
        mid = (lo + hi) // 2
        if history[mid][0] <= t:
            best = history[mid][1]
            lo = mid + 1
        else:
            hi = mid - 1
    return best


def frame(scenes, buses, t, total, title_f, label_f, small_f):
    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)

    # Which scene is on screen, and how far into it.
    start, scene, into = 0.0, scenes[0], t
    for s in scenes:
        if t >= start and t < start + s["len"]:
            scene, into = s, t - start
            break
        start += s["len"]

    d.text((48, 36), "bevy_siren", font=title_f, fill=INK)
    d.text((48, 78), "a 32-second tour of the crate's decisions, with the sound it made", font=small_f, fill=MUTED)

    # **The scene, large**, with its own explanation under it: this is the "what is it demoing" half.
    d.text((48, 150), scene["title"], font=title_f, fill=SCENE_INK)
    what = SCENE_WHAT.get(scene["title"], "")
    d.text((48, 200), what, font=label_f, fill=INK)
    if scene["rms"]:
        d.text((48, 238), f"peak {scene['peak']} dBFS   rms {scene['rms']} dBFS   (this scene, measured)", font=small_f, fill=MUTED)

    # **The bus meters**: the crate's own gains, one bar per bus, with the duck visible as a shorter bar.
    top = 320
    d.text((48, top - 34), "each bus's gain, from SirenMix — the crate's numbers, not the demo's", font=small_f, fill=MUTED)
    for i, (bus, colour) in enumerate(BUS_COLOUR.items()):
        y = top + i * 46
        db = bus_db_at(buses.get(bus, [(0.0, 0.0)]), t)
        # −30 dB is an empty bar; 0 dB is full.
        width = int(max(0.0, min(1.0, (db + 30.0) / 30.0)) * 700)
        d.text((48, y + 4), f"{bus:<9}", font=label_f, fill=colour)
        d.rectangle([170, y + 2, 170 + 700, y + 24], outline=(52, 60, 76))
        if width:
            d.rectangle([170, y + 2, 170 + width, y + 24], fill=colour)
        d.text((890, y + 4), f"{db:6.1f} dB", font=label_f, fill=colour)

    # **The timeline**: the whole tour, the scenes numbered, the playhead where the sound is.
    y = H - 92
    d.rectangle([48, y, W - 48, y + 34], outline=(52, 60, 76))
    left = 48
    for i, s in enumerate(scenes):
        w = (s["len"] / total) * (W - 96)
        if i == t // max(s["len"], 1e-9):
            pass
        d.text((left + 8, y + 9), str(i + 1), font=small_f, fill=MUTED)
        left += w
    playhead = 48 + (t / total) * (W - 96)
    d.rectangle([playhead - 2, y - 6, playhead + 2, y + 40], fill=SCENE_INK)
    elapsed = f"{int(t // 60)}:{t % 60:04.1f}"
    d.text((W - 150, y + 9), f"{elapsed} / {int(total // 60)}:{total % 60:04.1f}", font=small_f, fill=MUTED)
    return im


# **What each moment demonstrates**, in a sentence a stranger can read. Keyed by the report's own scene titles, so a
# renamed scene fails the build rather than shipping a blank caption.
SCENE_WHAT = {
    "1 · three footsteps": "one cue, three recordings — the picker never plays the same foot twice in a row",
    "2 · a burst": "one sample at an exact pitch: a play with no decision in it, so it takes no draw from the seed",
    "3 · a bed and a pad": "a looping ambience and a looping music layer, each on its own bus",
    "4 · a line ducks the music": "a line of dialogue lowers Music by 8 dB and Effects by 4, then releases",
    "5 · the layer changes on the bar": "the combat layer comes in on a bar of 100 bpm, not on a frame",
    "6 · a stinger": "a stinger arrives at once and ducks the music it plays over",
    "7 · the budget": "six hits against a cue limit of three: three play, three are taken back",
    "8 · distance": "the same sound at 4 m and at 16 m — the distance law decides the level",
    "9 · a mistake": "a cue name that does not exist: refused, and the console says which",
}


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    scenes = parse_report(src / "scenes.txt")
    buses = parse_bus(src / "bus-gain.csv")
    missing = [s["title"] for s in scenes if s["title"] not in SCENE_WHAT]
    if missing:
        print(f"no caption for {missing}", file=sys.stderr)
        return 1
    total = sum(s["len"] for s in scenes)
    title_f, label_f, small_f = font(34, True), font(20), font(15)

    out.parent.mkdir(parents=True, exist_ok=True)
    # Frames are piped to ffmpeg: 32 s at 30 fps is 960 frames, and keeping them on this volume is 1.5 GB of PNGs it does
    # not have (the volume sits at 100 %).
    cmd = [
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "pipe:0",
        "-i", str(src / "mix.wav"),
        "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "192k", "-ar", "48000",
        "-movflags", "+faststart", "-shortest", str(out),
    ]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    n = int(total * FPS)
    for i in range(n):
        proc.stdin.write(frame(scenes, buses, i / FPS, total, title_f, label_f, small_f).tobytes())
    proc.stdin.close()
    if proc.wait() != 0:
        print("ffmpeg failed", file=sys.stderr)
        return 1
    print(f"{out}: {n} frames, {total:.0f} s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
