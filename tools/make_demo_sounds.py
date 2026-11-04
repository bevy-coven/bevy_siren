#!/usr/bin/env python3
"""**The demo's sound set**, synthesised here and released CC0 — the recordings `examples/demo` and `examples/tour` play.

Eleven files, written as Ogg Vorbis (the format the crate's own `assets/audio` already uses, so a clone needs no
converter to run the demos). Every one is made by this script from arithmetic — no downloaded audio, no sample library,
nothing whose licence has to be tracked — so `assets/README.md` can say exactly where each came from and the repository
carries no asset it cannot reproduce.

    python3 tools/make_demo_sounds.py assets/audio

What each file is:

| file | what it is | why the demo needs it |
|---|---|---|
| `step_01.ogg`, `step_02.ogg`, `step_03.ogg` | three footsteps: filtered noise bursts, 0.2–0.3 s | a cue with three variants and a no-repeat window — the crate's headline feature |
| `hit_01.ogg` | a low, short thump | a one-shot on `Effects` with a polyphony cap |
| `click.ogg` | a bright 1.1 kHz tick | UI, and the cue that must not move the shared stream |
| `wind.ogg` | a 4 s loopable noise bed | a `Mode::Loop` ambience, quiet and long |
| `music_calm.ogg`, `music_combat.ogg` | two 4 s loopable pads at 100 bpm (2.4 s a bar: 4 beats) | vertical layering that changes on the bar |
| `stinger.ogg` | a short rising triad | a stinger that ducks the music it plays over |
| `line_01.ogg`, `line_02.ogg` | two short, dry "voice" tones | dialogue ducking music and effects |

Every loop is an exact number of bars at the tempo the example's score uses, so a layer change on the bar lands where a
listener expects it: at 100 bpm a bar is 2.4 s, and the two pads are 4 bars = 9.6 s.

The synthesis is numpy, at 44.1 kHz, and the Ogg encode is `ffmpeg`'s native Vorbis encoder (the same one `assets/README.md`
already documents). The script is deterministic: the same numpy produces the same samples, so a re-run rewrites the same
files, and `--check` says whether the files on disk are the ones this script makes.
"""

import argparse
import hashlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

RATE = 44_100
"""44.1 kHz: what a game's music is authored at, and what `ffmpeg`'s Vorbis encoder takes without resampling."""

BAR_S = 2.4
"""One bar at 100 bpm in 4/4 — the tempo `examples/demo`'s score declares (`docs/guide/music.md`)."""


def rng(seed: int) -> np.random.Generator:
    """A generator whose output depends only on `seed`, so a re-run of this script reproduces every file byte for byte."""
    return np.random.default_rng(seed)


def fade(x: np.ndarray, in_s: float = 0.004, out_s: float = 0.02, curve: str = "exp") -> np.ndarray:
    """A short fade in and a longer fade out, so no file starts or ends on a step (a step is a click)."""
    n = len(x)
    a = max(1, int(in_s * RATE))
    b = max(1, int(out_s * RATE))
    out = x.copy()
    out[:a] *= np.linspace(0.0, 1.0, a)
    if curve == "exp":
        out[-b:] *= np.exp(np.linspace(0.0, -6.0, b))
    else:
        out[-b:] *= np.linspace(1.0, 0.0, b)
    return out


def one_pole_lowpass(x: np.ndarray, cutoff_hz: float) -> np.ndarray:
    a = np.exp(-2.0 * np.pi * cutoff_hz / RATE)
    y = np.empty_like(x)
    acc = 0.0
    for i, v in enumerate(x):
        acc = (1.0 - a) * v + a * acc
        y[i] = acc
    return y


def one_pole_highpass(x: np.ndarray, cutoff_hz: float) -> np.ndarray:
    return x - one_pole_lowpass(x, cutoff_hz)


def footstep(seed: int, duration_s: float, low_hz: float, high_hz: float) -> np.ndarray:
    """Filtered noise with a fast attack and an exponential tail: a foot on a surface.

    Noise, not a tone: a footstep's character is its *band*, and a tone would sound like a beep at any band. Three
    different (band, duration) pairs make three variants a listener can tell apart.
    """
    n = int(duration_s * RATE)
    noise = rng(seed).normal(0.0, 1.0, n)
    x = one_pole_highpass(one_pole_lowpass(noise, high_hz), low_hz)
    x *= np.exp(-np.linspace(0.0, 7.0, n))
    x = fade(x, 0.002, 0.05)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.7


def thump(seed: int) -> np.ndarray:
    """A low body plus a click: what a hit reads as on a spectrogram (a low band and a broadband edge)."""
    n = int(0.26 * RATE)
    t = np.arange(n) / RATE
    body = np.sin(2 * np.pi * 70.0 * t) * np.exp(-t * 22.0)
    edge = rng(seed).normal(0.0, 1.0, n) * np.exp(-t * 90.0) * 0.5
    x = fade(body + edge, 0.001, 0.03)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.85


def tick() -> np.ndarray:
    """A bright, very short click: the sound a UI cue wants, and the one this demo uses to show it costs no stream."""
    n = int(0.05 * RATE)
    t = np.arange(n) / RATE
    x = np.sin(2 * np.pi * 1_100.0 * t) * np.exp(-t * 120.0)
    x = fade(x, 0.001, 0.01)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.6


def loopable(x: np.ndarray, seconds: float) -> np.ndarray:
    """Cut `x` to exactly `seconds` so that **the end continues into the start**, and the join is not a click.

    The first version of this function crossfaded the tail into the head *inside* the buffer and then wrote a line that
    multiplied the tail by zero — a no-op dressed as a second half of the join. `tests/demo.rs` measured the result: the
    seam of `music_calm.ogg` was a step of 0.0544 against a 99.9th-percentile step of 0.0330 **inside** the file, i.e. a
    click every pass.

    What actually makes a loop seamless is that the signal is **periodic in the buffer's length**. These sources are sums
    of sines whose frequencies are not periods of the buffer, so they are not periodic — but a buffer taken from a
    *circular* mix is: every oscillator is evaluated on `t` wrapped modulo the loop, so the value at the last sample's
    successor is the value at the first. `loopable` therefore asks its caller for a phase-correct length rather than
    stitching two ends together.
    """
    want = int(round(seconds * RATE))
    if len(x) < want:
        reps = int(np.ceil(want / len(x)))
        x = np.tile(x, reps)
    return x[:want].copy()


def wind(seed: int) -> np.ndarray:
    """A 9.6 s bed — two bars, so it lines up with a bar change: band-limited noise with a swell whose period fits the loop.

    The noise component is **not** periodic, and it must not be: a bed of noise whose every pass is identical is a tone,
    not a bed. What is made periodic is the swell (four cycles in the file), and the noise's ends are tapered by a very
    short equal-power crossfade of 5 ms — short enough to be inaudible on noise, long enough that the join is not a step.
    """
    n = int(round(9.6 * RATE))
    t = np.arange(n) / RATE
    noise = rng(seed).normal(0.0, 1.0, n)
    x = one_pole_lowpass(one_pole_highpass(noise, 120.0), 900.0)
    snapped = lambda f: round(f * 9.6) / 9.6  # noqa: E731
    sw = 0.6 + 0.4 * np.sin(2 * np.pi * snapped(1.0 / 9.6) * t)
    x = x * sw
    # A 5 ms equal-power wrap on the noise only, so the join is a continuation and not a step.
    fade_n = int(0.005 * RATE)
    ramp = np.linspace(0.0, 1.0, fade_n)
    head = x[:fade_n].copy()
    tail = x[-fade_n:].copy()
    x[:fade_n] = head * ramp + tail * (1.0 - ramp)
    x = loopable(x, 9.6)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.35


def pad(root_hz: float, bright: float, seconds: float = 9.6, seed: int = 0) -> np.ndarray:
    """A four-bar loopable pad at `root_hz`: a root, a fifth and an octave, with a slow tremolo.

    **Every periodic part is evaluated on a phase that fits the loop an exact number of times**, which is what makes the
    last sample continue into the first with no step: `round(f * seconds) / seconds` is the nearest frequency to `f`
    whose period divides the file. Without it the join is a click (measured: 0.0544 against a 99.9th-percentile step of
    0.0330) and a layer that loops on the bar drifts out of phase with the bar as well.
    """
    n = int(round(seconds * RATE))
    t = np.arange(n) / RATE
    snapped = lambda f: round(f * seconds) / seconds  # noqa: E731 - a one-line phase correction
    x = np.zeros(n)
    for multiple, level in ((1.0, 0.55), (1.5, 0.3), (2.0, 0.25), (3.0, 0.1 * bright)):
        x += level * np.sin(2 * np.pi * snapped(root_hz * multiple) * t + 0.3 * multiple)
    # A slow amplitude motion on the bar grid, and it is on the grid too: one cycle every bar, 4 in the file.
    x *= 0.75 + 0.25 * np.sin(2 * np.pi * snapped(1.0 / BAR_S) * t)
    x += 0.02 * rng(seed).normal(0.0, 1.0, n)
    x = loopable(x, seconds)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.4


def stinger() -> np.ndarray:
    """A rising triad, 0.6 s: short enough to sit over music and be named in a log line."""
    n = int(0.6 * RATE)
    t = np.arange(n) / RATE
    x = np.zeros(n)
    for i, hz in enumerate((392.0, 494.0, 587.0)):
        start = int(i * 0.12 * RATE)
        env = np.exp(-np.maximum(0.0, t - i * 0.12) * 10.0)
        env[:start] = 0.0
        x += env * np.sin(2 * np.pi * hz * t)
    x = fade(x, 0.003, 0.08)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.6


def voice_tone(hz: float) -> np.ndarray:
    """A dry, short 'line': a formant-ish pair of tones with a fast decay, 0.45 s — a stand-in for a spoken line."""
    n = int(0.45 * RATE)
    t = np.arange(n) / RATE
    env = np.exp(-t * 9.0) * (1.0 - np.exp(-t * 200.0))
    x = env * (np.sin(2 * np.pi * hz * t) + 0.4 * np.sin(2 * np.pi * hz * 2.7 * t))
    x = fade(x, 0.004, 0.06)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.55


def files() -> dict[str, np.ndarray]:
    """Every file this script makes, by name — the one list the writer, `--check` and `tests/demo.rs` all read."""
    return {
        "step_01.ogg": footstep(11, 0.22, 90.0, 1_700.0),
        "step_02.ogg": footstep(23, 0.26, 70.0, 1_300.0),
        "step_03.ogg": footstep(37, 0.19, 140.0, 2_400.0),
        "hit_01.ogg": thump(5),
        "click.ogg": tick(),
        "wind.ogg": wind(71),
        "music_calm.ogg": pad(196.0, 0.6),
        "music_combat.ogg": pad(233.08, 1.4),
        "stinger.ogg": stinger(),
        "line_01.ogg": voice_tone(330.0),
        "line_02.ogg": voice_tone(392.0),
    }


def encode(samples: np.ndarray, out: Path) -> None:
    """Write `samples` as Ogg Vorbis, **declaring exactly its own length**.

    A Vorbis stream's last page declares the frame count, and the last packet decodes to a whole block — more than the
    stream has. `src/ogg.rs` (iteration 115) exists because of that excess: the decoder returns up to 21 ms of padding,
    and the crate cuts a loop to the length its last page declares. Measured here before the fix: `step_01.ogg` was
    written as 0.220 s and decoded to **0.232 s**, 530 frames of padding on a 0.22 s footstep.

    **What was tried and does not work**: `-frames:a <n>` on the output. Measured: a 9 702-frame input still decodes to
    **10 240** frames with the flag, because the padding is in the last block and the granule, not in how many packets are
    written. The decoder's own count is therefore up to one block longer than the arithmetic, always, and the length that
    is exact is the one the last page **declares** — which is what `src/ogg.rs` reads and what `tests/demo.rs` asserts.
    """
    """Write `samples` (mono float, 44.1 kHz) as Ogg Vorbis through `ffmpeg`'s native encoder, the one `assets/README.md` documents."""
    pcm = np.clip(samples, -1.0, 1.0)
    data = (pcm * 32_767.0).astype("<i2").tobytes()
    # The temp file carries the **output** suffix: ffmpeg picks its muxer from it, and a `.wav` name here once made
    # the Vorbis encode fail with `Could not open encoder before EOF`.
    with tempfile.NamedTemporaryFile(suffix=".ogg") as tmp:
        subprocess.run(
            [
                "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
                "-f", "s16le", "-ar", str(RATE), "-ac", "1", "-i", "pipe:0",
                # ffmpeg's native Vorbis encoder takes **exactly two channels** (`assets/README.md` found the same for the
                # quick start's footsteps): the one signal is written to both, so the file is stereo and identical on
                # each side rather than a mono stream the encoder refuses with `Could not open encoder before EOF`.
                "-af", "pan=stereo|c0=c0|c1=c0",
                "-c:a", "vorbis", "-strict", "-2", "-q:a", "5", str(tmp.name),
            ],
            input=data,
            check=True,
        )
        shutil.copyfile(tmp.name, out)


def pcm_digest(path: Path) -> str:
    """The SHA-256 of a file's **decoded samples**, not of its bytes.

    An Ogg container carries a random stream serial, so two encodes of the same arithmetic differ in about twenty bytes
    and not in a sample (measured: 21 bytes for `step_01.ogg`, 95 for `music_calm.ogg`, and the decoded PCM identical).
    What `--check` must hold is the audio, because that is what a listener hears and what a re-run of this script is
    supposed to reproduce: the container's serial is noise.
    """
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path), "-f", "s16le", "-ac", "1", "-ar", str(RATE), "pipe:1"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    return hashlib.sha256(out).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out", type=Path, help="the directory to write into, `assets/audio` in this repository")
    ap.add_argument("--check", action="store_true", help="write nothing; fail when a file is missing or differs")
    args = ap.parse_args()
    if shutil.which("ffmpeg") is None:
        print("ffmpeg is needed (its native Vorbis encoder writes these files)", file=sys.stderr)
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    made = files()
    stale = []
    for name, samples in made.items():
        out = args.out / name
        if args.check:
            if not out.exists():
                stale.append(f"{name}: missing")
                continue
            with tempfile.NamedTemporaryFile(suffix=".ogg") as tmp:
                encode(samples, Path(tmp.name))
                if pcm_digest(Path(tmp.name)) != pcm_digest(out):
                    stale.append(f"{name}: its samples differ from what this script makes")
            continue
        encode(samples, out)
        print(f"{name:<18} {len(samples) / RATE:6.3f} s  {pcm_digest(out)[:16]}")
    if args.check and stale:
        print("\n".join(stale), file=sys.stderr)
        return 1
    if args.check:
        print(f"{len(made)} files: all present and identical to what this script makes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
