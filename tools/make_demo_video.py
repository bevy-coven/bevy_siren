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
    """The vignettes: title, the two halves' numbers, and what to listen for — from `demo_render`'s own report."""
    scenes, current = [], None
    for line in path.read_text().splitlines():
        if line.startswith("vignette "):
            title = line.split("·", 1)[1].strip()
            if current is None or current["title"] != title:
                current = {"title": title, "halves": [], "listen": ""}
                scenes.append(current)
        elif current is not None and line.strip().startswith(("a —", "b —")):
            part = line.strip()[0]
            seg = line.split(":", 1)[1]
            length = float(seg.split(",")[0].split()[0])
            rms = float(line.split("rms ")[1].split()[0]) if "rms " in line else -40.0
            current["halves"].append({"which": part, "label": line.strip()[4:].split(":")[0], "len": length, "rms": rms})
        elif current is not None and line.strip().startswith("listen for: "):
            current["listen"] = line.strip()[len("listen for: "):]
    return [s for s in scenes if len(s["halves"]) == 2]


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


def frame(scenes, wave_paths, t, total, title_f, label_f, small_f, big_f):
    """One frame: which vignette, which half (**A** or **B**), what to listen for, and the level of both halves so far."""
    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)

    # Which vignette and which half is playing, from the report's own lengths plus the gaps the renderer inserted.
    at, idx, half = 0.0, 0, 0
    for i, v in enumerate(scenes):
        for h, part in enumerate(v["halves"]):
            if at <= t < at + part["len"] + 0.35:
                idx, half = i, h
                break
            at += part["len"] + (0.35 if h == 0 else 0.9)
        else:
            continue
        break
    v = scenes[idx]
    part = v["halves"][half]

    d.text((48, 30), "bevy_siren", font=title_f, fill=INK)
    d.text((48, 70), f"{len(scenes)} A/B pairs — one variable each, level-matched, the sound the crate made", font=small_f, fill=MUTED)

    # **A or B, large.** The single most important thing on the frame: a listener must know which one they are hearing.
    tag = "A" if half == 0 else "B"
    colour = (255, 120, 120) if half == 0 else (120, 220, 160)
    d.rectangle([48, 140, 130, 240], outline=colour, width=4)
    d.text((66, 158), tag, font=big_f, fill=colour)
    d.text((150, 148), part["label"], font=title_f, fill=INK)
    d.text((150, 196), f"vignette {idx + 1}: {v['title']}", font=label_f, fill=colour)

    # **What to listen for**, wrapped: the half of the demo people said was missing.
    words, line, lines = v["listen"].split(), "", []
    for word in words:
        trial = (line + " " + word).strip()
        if d.textlength(trial, font=label_f) > W - 300:
            lines.append(line)
            line = word
        else:
            line = trial
    lines.append(line)
    for i, text in enumerate(lines[:4]):
        d.text((150, 250 + i * 30), text, font=label_f, fill=INK)

    # **The two halves as waveforms**, which is the one picture that shows *why* A and B differ: eight even hits against
    # an uneven walk, forty spikes against four. Read from the WAVs the renderer wrote, one column per pixel.
    wave_top = 380
    d.text((48, wave_top - 24), "the two halves, sample for sample (A above, B below)", font=small_f, fill=MUTED)
    for h, part_name in enumerate(("a", "b")):
        path = wave_paths.get((idx, part_name))
        if path is None or not path.exists():
            continue
        raw = subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path), "-f", "s16le", "-ac", "1", "-ar", "8000", "pipe:1"],
            check=True, stdout=subprocess.PIPE,
        ).stdout
        data = memoryview(raw).cast("h")
        n = len(data)
        y0, hgt = wave_top + h * 70, 62
        c = (255, 120, 120) if h == 0 else (120, 220, 160)
        d.line([(48, y0 + hgt // 2), (W - 48, y0 + hgt // 2)], fill=(40, 46, 58))
        for x in range(W - 96):
            lo, hi = n * x // (W - 96), n * (x + 1) // (W - 96)
            if hi <= lo:
                continue
            seg = data[lo:hi]
            peak = max(max(seg), -min(seg)) / 32768.0
            peak = peak ** 0.5  # a square root, so a quiet hit is visible beside a loud one
            amp = int(peak * (hgt // 2 - 2))
            d.line([(48 + x, y0 + hgt // 2 - amp), (48 + x, y0 + hgt // 2 + amp)], fill=c)

    # **The two halves' levels**, so "matched" is visible: two bars, A and B, at their RMS.
    base = H - 128
    d.text((48, base - 26), "the two halves, matched to within 0.5 LU (ITU-R BS.1770)", font=small_f, fill=MUTED)
    for h, p_ in enumerate(v["halves"]):
        y = base + h * 34
        rms = p_.get("rms", -40)
        width = int(max(0.05, min(1.0, (rms + 60) / 60)) * 700)
        c = (255, 120, 120) if h == 0 else (120, 220, 160)
        d.text((48, y + 4), "A" if h == 0 else "B", font=label_f, fill=c)
        d.rectangle([90, y + 1, 90 + 700, y + 22], outline=(52, 60, 76))
        d.rectangle([90, y + 1, 90 + width, y + 22], fill=c)
        d.text((810, y + 4), f"{rms:6.1f} dBFS", font=label_f, fill=c)

    elapsed = f"{int(t // 60)}:{t % 60:04.1f}"
    d.text((W - 170, H - 46), f"{elapsed} / {int(total // 60)}:{total % 60:04.1f}", font=small_f, fill=MUTED)
    return im


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    scenes = parse_report(src / "tour.txt")
    # Each half's own WAV, for the waveform: `vignette-<n>-<a|b>.wav`.
    wave_paths = {(i, h): src / f"vignette-{i + 1}-{h}.wav" for i in range(len(scenes)) for h in ("a", "b")}
    total = sum(h["len"] for s in scenes for h in s["halves"]) + 0.35 * len(scenes) + 0.9 * len(scenes)
    title_f, label_f, small_f, big_f = font(34, True), font(20), font(15), font(64, True)

    out.parent.mkdir(parents=True, exist_ok=True)
    # Frames are piped to ffmpeg: 32 s at 30 fps is 960 frames, and keeping them on this volume is 1.5 GB of PNGs it does
    # not have (the volume sits at 100 %).
    cmd = [
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "pipe:0",
        "-i", str(src / "tour.wav"),
        "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "192k", "-ar", "48000",
        "-movflags", "+faststart", "-shortest", str(out),
    ]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    n = int(total * FPS)
    try:
        for i in range(n):
            proc.stdin.write(frame(scenes, wave_paths, i / FPS, total, title_f, label_f, small_f, big_f).tobytes())
        proc.stdin.close()
    except BrokenPipeError:
        # **Write the encoder's own words**, not a Python traceback: a broken pipe means ffmpeg stopped, and why is in its
        # stderr (a missing input, a bad argument), which is the only thing worth printing.
        proc.stdin.close()
        print("ffmpeg stopped early:\n" + proc.stderr.read().decode(errors="replace"), file=sys.stderr)
        proc.wait()
        return 1
    if proc.wait() != 0:
        print("ffmpeg failed", file=sys.stderr)
        return 1
    print(f"{out}: {n} frames, {total:.0f} s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
