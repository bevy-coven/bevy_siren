# Iteration 95 — review

## Pass 1 — function

**Does it do what the source and the case say?** The case is "decode throughput fell to 419.8 / 568.8 clips/s". It is now
**698.6 / 766.2 / 784.0 across three runs**. The A/B on the analysis pair is **9.44 ms → 5.11 ms, −45.9 %**, which is one
1024-point FFT pass over 400 frames — the size the duplication predicted (`research.md`, the `O(N log N)` row).

**Do the outputs change?** No, and this is proven two ways rather than asserted:

- the existing **10 spectral** and **13 tempo** tests pass with **no edit to any expectation**, including the 80-tempo
  corpus that pins accuracy to ±2 BPM;
- two new parity tests compare the shared and solo entry points **field for field** — `Tempo`'s `bpm`, `confidence` and
  `octave_ambiguous`; `Spectral`'s `centroid_hz`, `rolloff_hz` and `noise_floor_db` — on a rhythmic and a non-rhythmic
  buffer. Both pass.
- all **21 rendered wavs are byte-identical** to iteration 94's (`cmp` over every file in `evidence/after`).

**Verdict: GREAT.**

## Pass 2 — zoom

Read: `spectrograms/cue-bang-0-spectrum.png`, its `-tail.png`, `cue-footstep-grass-0-spectrum.png`,
`cue-music-combat-0-spectrum.png`.

- `cue-bang-0-spectrum.png`: attack at 0–0.03 s, full bandwidth to 11 kHz, decaying into five sustained partials. **No
  vertical line**, which is what a click from a misaligned frame boundary would look like. The partials hold flat to
  0.33 s with no hole and no shelf.
- `cue-bang-0-tail.png` (last 0.1 s): the partials continue to the edge of the frame and stop; the broadband energy has
  fully decayed by 0.035 s. **No denormal tail** — the noise floor falls to the colourbar's bottom and stays there rather
  than hovering as a visible haze.
- Loudness unchanged: **−21.5 LUFS integrated, −7.5 dBFS true peak** on the master, identical to iteration 93 and 94.
  `voices-8.wav` peaks at 0.000 dBFS and −12.68 dB RMS, also unchanged.

A refactor of an offline analyser cannot put a click in a rendered cue — the render path does not call it — and the byte
comparison is the stronger statement: the images and the audio are not merely similar, they are the same bytes.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Regenerated with a different volume (300 clips instead of 200) and read every check again:

- `indexbench` at 300: 766.2 / 784.0 / 698.6 clips/s, `5000 clips at this rate: 9 s`.
- The `analysis` module's 5 tests and the two parity tests hold under the same code path a fresh index uses.
- Can a developer understand it without the code? The module doc opens with the table of which iteration added which pass
  and what each computed per frame, then states the measurement. A reader sees *why* the file exists before *what* it
  does.

**One criticism, recorded rather than fixed:** `mel::log_mel` still computes its own transform. That is visible to a
reader of the new module's doc, which says so explicitly, and it is the next item — but a reader could reasonably ask why
the share stops at two of three. The answer (`log_mel` reflect-pads to a different frame count, so its frames are not
these frames) is in the doc and in `perf.md`. Sharing it is a change to *what is computed*, not to *how often*, and it
needs its own evidence.

**Verdict: GREAT.**
