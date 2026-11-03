# review.md — iteration 41, three passes

## Pass 1 — Function: does it do what the source and the case say?

**The case.** `cues.yaml` 14: *"Distance decides the gain … 0 dB at the near radius and −6.02 dB at twice it, each within
0.01"* (`source: inverse-distance falloff`). `cues.yaml` 13: *"a spatial play with no ears is refused, carrying the
position"*. S1: the default is `Inverse { near_m: 4, rolloff: 1 }`, *"full at the listener's feet and halves at 8 m"*.

**Measured on the finished panel** (`evidence/panel/spatial-95.png`, 1280×720, frame 95):

| Where | Text |
|---|---|
| the cue row | `bang  Effects  once  +0.0 dB  priority 30  inverse at 8 m -6.02 dB` |
| the log | `0.51s played \`bang\` (hit_01) on Effects at -6.02 dB` |
| `render.txt` | `spatial: 1.000 at 4 m, 0.500 at 8 m, 0.250 at 16 m` |
| `render.txt` | `refusal-bang-no-SpatialListener-to-place-a-sound-at--3--0--4-: no SpatialListener … [3, 0, 4]` |

Three places state −6.02, one states the whole law 1 / 0.5 / 0.25, and the refusal carries its position. Case 14's
assertion (`tests/play.rs`, −6.0206 within 1e-3) is now readable off the picture.

**The precision fix, measured.** Before: the row (had it existed) would read `-6.02` beside a log line saying `at -6.0 dB`
— and `-6.0` is also what a cue authored `gain_db: -6.0` produces, so the log could not distinguish a distance from a
fader. After: `-6.02` in both. The new test asserts both halves — the sentence's two decimals and the field's full
precision — and the mutation (`{:.1}`) reddens it.

**Verdict: GREAT.** The case's number is on screen, twice, from two code paths, and the refusal is recorded.

## Pass 2 — Zoom: read the plots and crops closely

Read: `evidence/panel/{spatial-95,spatial-95-1920,zoom-cue-row,zoom-log,fresh-70}.png`,
`evidence/spectrograms/{bang-spectrum,bang-wave,bang-attack,cancel-spectrum}.png`,
`evidence/plots/bang-distance.svg`, the shipped `docs/media/mixer.png` and `mixer.gif`.

**The plot.** `bang-distance.svg`: flat from the left edge to the 4 m line, then a straight descent — −6.02 at 8 m,
−12.04 at 16 m, one halving per doubling. Both the computed curve and the three measured points sit on the same line, and
the 8 m marker coincides with the −6.02 gridline. No kink at `near_m`, which is where an off-by-one in the `max(0, d −
near)` clamp would show.

**The waveform.** `bang-wave.png` is a clean exponential decay with no gap, no shelf and no hard tail; `bang-spectrum.png`
is a wideband block ending at 0.03 s. **That block is the fixture, not a defect**: `examples/render`'s `wave()` is
`sin(2πft)·e^{−12t}` at 440 Hz over 320 ms, and the same block appears in every one-shot the harness writes. Confirmed by
`cancel-spectrum.png`, whose cue differs only in its authored gain.

**The amplitude check that isolates the law.** `bang` and `ui.cancel` share the harness's fixture exactly — 320 ms, 440 Hz,
decay 12 — and differ only in gain: `bang` is 0 dB × the 8 m law, `ui.cancel` is authored −6.0 dB. Measured peaks:
**−6.087 dB** and **−12.087 dB** — **exactly 6.00 dB apart**. A wrong law would put `bang` at a different offset.

**A defect this pass found and fixed.** With the new eighth cue row, the log's seventh line clipped: measured, the lowest
ink sat at **`y = 719` of 719 — 0 px**, the very defect iteration 32 fixed. The count is now six, and the margin is
**9 px**. The alternative — dropping the spatial cue to keep seven lines — would hide the only placed sound in the panel.

**Verdict: GREAT.** After the fix: no clip, no gap, and the law's plot and its measurements agree.

## Pass 3 — Fresh eyes: a different seed and a different script

`SIREN_SEED=4242 SIREN_KEYS=spatial,duck,stinger SIREN_KEY_EVERY=25`, frame 70 (`evidence/panel/fresh-70.png`). Different
seed, different script, different frames.

The spatial row is unchanged — `inverse at 8 m -6.02 dB` — because it is a property of the **table**, not of the run,
which is the right behaviour: a reader should not have to catch the right frame to learn a cue's law. The log carries
`0.34s played \`bang\` (hit_01) on Effects at -6.02 dB` under a different seed, and the header's `4 voices` matches the bus
column's `2 + 1 + 0 + 1`.

**Can a developer understand it without the code?** Yes. The row says the model by name (`inverse`), the distance
(`8 m`) and the result (`-6.02 dB`); the log repeats the result; `docs/guide/spatial.md` gives the formula with the same
three names. Nothing in the picture needs the source.

**Verdict: GREAT.**
