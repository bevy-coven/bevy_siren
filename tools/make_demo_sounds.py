#!/usr/bin/env python3
"""**The demo's sound set** — rewritten, after the first version was heard and was not good enough.

## What was wrong (measured, not guessed)

The first set had a **static drone** under it: `music_calm`/`music_combat` were three pure sine waves (196, 294, 392 Hz;
spectral flatness **0.0005** — 0 is pure tones, 1 is noise) and `wind` was a permanent broadband noise bed. Both played
from the third scene on, so the tour's noise floor **never dropped below −19 dBFS for the last 27 of its 32 seconds**, and
66 % of the pad's energy sat below 200 Hz. Heard back, that is a hum — a listener called it *"cosmic background
radiation"* — with no note onsets and nothing that ever stops.

## What the rules are

From the reading that followed (`docs/loop/runs/2026-11-04-125/research.md`): a demo is judged on whether a difference is
**audible and attributable**, and loudness matching within **0.5 LUFS** is the price of admission for any A/B; a reel
should be about a minute and should **not** carry a continuous music bed, because it masks the thing being demonstrated.
So this set is built to have **silence between events**:

| file | what it is | why |
|---|---|---|
| `step_01..03.ogg` | three filtered-noise footsteps, 0.19–0.26 s | short, distinct, and they *end* |
| `hit_01.ogg` | a low thump with a click edge | a transient, not a tone |
| `click.ogg` | a 1 kHz tick, 50 ms | a UI sound |
| `note_low.ogg`, `note_mid.ogg`, `note_high.ogg` | **plucked** notes with harmonic decay, 0.6 s | a pluck has an attack and a decay; an organ tone does not |
| `bass.ogg` | a plucked low note, 0.9 s | the low end, without a drone |
| `music_calm.ogg`, `music_combat.ogg` | **rhythmic phrases** — plucks on a beat grid, 4 bars at 100 bpm — with silence between notes | a phrase leaves gaps, so a duck is audible and nothing hums |
| `stinger.ogg` | a plucked three-note riser, 0.7 s | it ducks the music for a second, so a drone there would be the loudest one |
| `line_01.ogg`, `line_02.ogg` | short dry two-partial tones | a stand-in for a spoken line |

No sustained tone, no noise bed, no `wind.ogg`. The music is a *figure*, so between notes the file is silent and a change
made on a bar is heard as a change, not as a swell under a wash.

Deterministic: the same numpy produces the same samples, and `--check` compares the **decoded** samples (an Ogg container
carries a random stream serial, so two encodes differ in about twenty bytes and in no sample).
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
"""One bar at 100 bpm in 4/4 — the tempo the demo's score declares."""


def rng(seed: int) -> np.random.Generator:
    return np.random.default_rng(seed)


def fade(x: np.ndarray, in_s: float = 0.003, out_s: float = 0.02, curve: str = "exp") -> np.ndarray:
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
    """Filtered noise with a fast attack and an exponential tail: a foot on a surface, and then silence."""
    n = int(duration_s * RATE)
    noise = rng(seed).normal(0.0, 1.0, n)
    x = one_pole_highpass(one_pole_lowpass(noise, high_hz), low_hz)
    x *= np.exp(-np.linspace(0.0, 7.0, n))
    x = fade(x, 0.002, 0.05)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.7


def thump(seed: int) -> np.ndarray:
    """A low body plus a click: what a hit reads as on a spectrogram — and it decays to nothing."""
    n = int(0.26 * RATE)
    t = np.arange(n) / RATE
    body = np.sin(2 * np.pi * 70.0 * t) * np.exp(-t * 22.0)
    edge = rng(seed).normal(0.0, 1.0, n) * np.exp(-t * 90.0) * 0.5
    x = fade(body + edge, 0.001, 0.03)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.85


def tick() -> np.ndarray:
    """A bright, very short click: the sound a UI cue wants."""
    n = int(0.05 * RATE)
    t = np.arange(n) / RATE
    x = np.sin(2 * np.pi * 1_100.0 * t) * np.exp(-t * 120.0)
    x = fade(x, 0.001, 0.01)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.6


def pluck(hz: float, seconds: float, seed: int = 0, bright: float = 1.0) -> np.ndarray:
    """**A plucked note**, not an organ tone.

    A pluck is what makes a demo's music stop between notes: the amplitude falls with `exp(-t·d)` and **the higher
    partials fall faster than the fundamental** (`d·k^0.6` for partial `k`), which is what the ear hears as a string
    being struck rather than a tone being held. The first version of this set used `sin(2πft)` with one slow envelope —
    a drone — and it was audible as a hum on every second after the third.

    `bright` scales how much of the upper partials is present: the combat layer is brighter, the calm one darker, so the
    two layers differ by *timbre* and not only by pitch.
    """
    n = int(seconds * RATE)
    t = np.arange(n) / RATE
    x = np.zeros(n)
    for k in range(1, 9):
        # Higher partials decay faster, which is the whole difference between a pluck and a held tone.
        decay = 6.0 * (k ** 0.6)
        level = (0.9 / k) * (bright ** (k - 1))
        x += level * np.sin(2 * np.pi * hz * k * t + 0.2 * k) * np.exp(-t * decay)
    # A short noise transient at the onset: the pluck's "attack", which a sum of sines alone does not have.
    n_attack = int(0.012 * RATE)
    if n_attack:
        burst = rng(seed).normal(0.0, 1.0, n_attack) * np.exp(-np.linspace(0.0, 5.0, n_attack)) * 0.25
        x[:n_attack] += burst
    x = fade(x, 0.0005, 0.02)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.8


def phrase(notes, seconds: float, seed: int = 0, bright: float = 1.0) -> np.ndarray:
    """Lay plucked notes on a beat grid and return exactly `seconds` of buffer — **with silence between them**.

    `notes` is `[(beat, hz, length_beats), …]`; a beat is `BAR_S / 4` at 100 bpm. The buffer is exactly four bars, so the
    phrase loops on the bar, and because every note decays the gaps are real silence rather than a floor.
    """
    n = int(round(seconds * RATE))
    out = np.zeros(n)
    beat = BAR_S / 4.0
    for i, (at_beat, hz, length) in enumerate(notes):
        note = pluck(hz, length * beat, seed=seed + i, bright=bright)
        start = int(at_beat * beat * RATE)
        end = min(n, start + len(note))
        if start < n:
            out[start:end] += note[: end - start]
    return out


def stinger() -> np.ndarray:
    """A short **plucked** riser: three notes up, each decaying, 0.7 s. The first version was three sustained sines held
    under an envelope — another drone — and the stinger ducks the music for a second, so a drone there was the most
    audible one of all."""
    n = int(0.7 * RATE)
    out = np.zeros(n)
    for i, hz in enumerate((392.0, 494.0, 587.0)):
        note = pluck(hz, 0.35, seed=70 + i, bright=1.5)
        start = int(i * 0.09 * RATE)
        end = min(n, start + len(note))
        out[start:end] += note[: end - start]
    return out / (np.max(np.abs(out)) + 1e-9) * 0.7


def voice_tone(hz: float) -> np.ndarray:
    """A dry, short 'line': a formant-ish pair of tones with a fast decay, 0.45 s — a stand-in for a spoken line."""
    n = int(0.45 * RATE)
    t = np.arange(n) / RATE
    env = np.exp(-t * 9.0) * (1.0 - np.exp(-t * 200.0))
    x = env * (np.sin(2 * np.pi * hz * t) + 0.4 * np.sin(2 * np.pi * hz * 2.7 * t))
    x = fade(x, 0.004, 0.06)
    return x / (np.max(np.abs(x)) + 1e-9) * 0.55


def files() -> dict[str, np.ndarray]:
    """Every file this script makes, by name — the list the writer, `--check` and `tests/demo.rs` all read."""
    # Four bars at 100 bpm. Calm: a bass note on each bar, a sparse pluck above it.
    calm = [
        (0.0, 98.0, 1.6),
        (4.0, 98.0, 1.6),
        (8.0, 87.3, 1.6),
        (12.0, 87.3, 1.6),
        (2.0, 392.0, 0.9),
        (6.0, 349.2, 0.9),
        (10.0, 392.0, 0.9),
        (14.0, 261.6, 1.2),
    ]
    # Combat: twice the notes, an octave up, brighter — the same grid, so the change lands on a bar.
    combat = [
        (0.0, 116.5, 0.9),
        (2.0, 116.5, 0.9),
        (4.0, 103.8, 0.9),
        (6.0, 103.8, 0.9),
        (8.0, 116.5, 0.9),
        (10.0, 116.5, 0.9),
        (12.0, 98.0, 0.9),
        (14.0, 98.0, 0.9),
        (1.0, 466.2, 0.4),
        (3.0, 466.2, 0.4),
        (5.0, 415.3, 0.4),
        (7.0, 415.3, 0.4),
        (9.0, 466.2, 0.4),
        (11.0, 466.2, 0.4),
        (13.0, 392.0, 0.4),
        (15.0, 392.0, 0.4),
    ]
    return {
        "step_01.ogg": footstep(11, 0.22, 90.0, 1_700.0),
        "step_02.ogg": footstep(23, 0.26, 70.0, 1_300.0),
        "step_03.ogg": footstep(37, 0.19, 140.0, 2_400.0),
        "hit_01.ogg": thump(5),
        "click.ogg": tick(),
        "note_low.ogg": pluck(98.0, 0.9, seed=3, bright=0.7),
        "note_mid.ogg": pluck(196.0, 0.6, seed=4),
        "note_high.ogg": pluck(392.0, 0.5, seed=5, bright=1.3),
        "bass.ogg": pluck(73.4, 0.9, seed=6, bright=0.5),
        "music_calm.ogg": phrase(calm, 4 * BAR_S, seed=20, bright=0.8),
        "music_combat.ogg": phrase(combat, 4 * BAR_S, seed=40, bright=1.4),
        "stinger.ogg": stinger(),
        "line_01.ogg": voice_tone(330.0),
        "line_02.ogg": voice_tone(392.0),
    }


def encode(samples: np.ndarray, out: Path) -> None:
    """Write `samples` as Ogg Vorbis through `ffmpeg`'s native encoder.

    **Two channels, always**: ffmpeg's native Vorbis encoder takes exactly two and fails `Could not open encoder before
    EOF` on one (`assets/README.md` found the same for the quick start's footsteps). And the temp file needs the
    **output** suffix — ffmpeg picks its muxer from the name, and a `.wav` name once made the encode fail.
    """
    pcm = np.clip(samples, -1.0, 1.0)
    data = (pcm * 32_767.0).astype("<i2").tobytes()
    with tempfile.NamedTemporaryFile(suffix=".ogg") as tmp:
        subprocess.run(
            [
                "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
                "-f", "s16le", "-ar", str(RATE), "-ac", "1", "-i", "pipe:0",
                "-af", "pan=stereo|c0=c0|c1=c0",
                "-c:a", "vorbis", "-strict", "-2", "-q:a", "5", str(tmp.name),
            ],
            input=data,
            check=True,
        )
        shutil.copyfile(tmp.name, out)


def pcm_digest(path: Path) -> str:
    """The SHA-256 of a file's **decoded samples**, not of its bytes — the container's serial is noise, the audio is not."""
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path), "-f", "s16le", "-ac", "1", "-ar", str(RATE), "pipe:1"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    return hashlib.sha256(out).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out", type=Path, help="the directory to write into, `assets/audio/demo` in this repository")
    ap.add_argument("--check", action="store_true", help="write nothing; fail when a file is missing or differs")
    args = ap.parse_args()
    if shutil.which("ffmpeg") is None:
        print("ffmpeg is needed (its native Vorbis encoder writes these files)", file=sys.stderr)
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    made = files()
    stale, extra = [], []
    # **A file no cue names is a leftover**, and the first version of this set left `wind.ogg` behind after the noise
    # bed was removed. `--check` reports it, so the folder cannot drift from the list.
    for path in sorted(args.out.glob("*.ogg")):
        if path.name not in made:
            extra.append(f"{path.name}: in the folder and not made by this script")
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
    problems = stale + extra
    if args.check and problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    if args.check:
        print(f"{len(made)} files: all present and identical to what this script makes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
