---
name: game-audio-judging
description: Judge game audio you cannot hear — render cues, buses and mixes to WAV, then read spectrograms, EBU R128 loudness and gain plots to find clicks, clipping, gaps, denormal tails and broken ducks. Use when changing a sound crate, a mixer, a ducking curve or an audio panel, and when a review demands evidence you cannot listen for.
---

# Judging game audio without an ear

You cannot hear the result, so every claim about how it sounds has to become a number or a picture a person can check.
This is the procedure, and the four traps that have already cost an iteration each.

## 1. Render to files first

Never judge a live mixer or a panel. Write the audio out:

- **One file per cue and variant**, at the gain and speed the play actually uses — read them from the same function
  the play system calls, not a copy of the formula.
- **A refusal is not a sound.** Write it as text (`.txt`) naming the reason; a refused cue that renders silence hides
  the bug it was supposed to surface.
- **One file per bus**, being the bus's *gain over time*, and one `mix.wav` of the sum.
- **`voices-<n>.wav` at 8, 64 and 256** voices for the cost and clipping checks.

16-bit mono 22.05 kHz is enough for `ffmpeg`'s analysis and small enough to keep in an evidence directory. If the
crate has an engine-free arithmetic module, drive **that** rather than spinning up a full ECS app: it makes the
harness fast, headless, and honest about which decisions it is checking.

## 2. Four traps, each already paid for

1. **A bus gain is a step function between frames.** Writing one sample per frame produces a 4 ms file that looks like
   a failure. Repeat each frame's gain for the samples that frame lasts: at 30 fps and 22.05 kHz that is 735 samples.
2. **A duck envelope takes the time since the duck fired, not the score's clock.** Feed `active_duck_db`-style functions
   an elapsed wall clock and the duck sits at its depth forever. The clock object that stores a start instant and
   subtracts it is the thing that gets this right.
3. **`showwavespic` on a gain curve draws a blob.** A gain is data, not audio: plot it as a **line** (a small script
   writing SVG is exact and dependency-free). Reserve `showwavespic` for actual audio.
4. **`ebur128` reads −70 LUFS for any clip shorter than one gating block (0.4 s), whatever its level.** A 0.32 s cue at
   −12 dBFS peak reads as silence because the gate has nothing to integrate. Measure short cues **looped**
   (`-af "aloop=loop=3:size=<samples>,ebur128"`) and measure amplitude with `astats` (`Peak level dB`,
   `RMS level dB`). A −70 in a loudness table means the clip was short, not quiet.

## 3. The pictures to look at

```sh
# Whole sample: spectrum with a log scale, and the waveform.
ffmpeg -y -i cue.wav -lavfi "showspectrumpic=s=900x300:legend=1:scale=log:color=intensity" spectrum.png
ffmpeg -y -i cue.wav -lavfi "showwavespic=s=900x200:colors=white" wave.png

# The two places defects live: the attack and the tail.
ffmpeg -y -i cue.wav -lavfi "atrim=0:0.05,showspectrumpic=s=900x300" attack.png
ffmpeg -y -sseof -0.1 -i cue.wav -lavfi "showspectrumpic=s=900x300" tail.png

# Loudness, integrated and true peak.
ffmpeg -nostats -i mix.wav -af ebur128=peak=true -f null - 2>&1 | tail -20
```

Read every image. What each defect looks like:

| Defect | How it looks |
|---|---|
| a click | a vertical smear across all frequencies at the attack or a gain step |
| clipping | a flat shelf at the top of the scale, and a true peak above 0 dBFS |
| a gap | a hole in the middle of the file |
| a denormal tail | a faint floor that never fades, or a ledge where the decay should be smooth |
| a broken duck | a gain plot that leaves 0 dB, never reaches its depth, overshoots, or never returns |

## 4. Judge a mix against a budget, not only a formula

Summing `n` voices at their authored gains will exceed full scale; that is the **voice budget's** job, not the gain's.
Say how many voices an evidence file holds, or a reviewer reads a budget's failure as a mixing defect.

## 5. Report

For each item: the picture or number, the defect it shows, and the earliest pass a fix would return to. A verdict with
no file behind it is an impression, and an impression is not evidence.
