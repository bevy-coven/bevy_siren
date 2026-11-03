# review.md — iteration 3, three passes

## Pass 1 — function

**The two claims.**

- **A.** `SirenSettings::seed` is the stream the crate draws from: two projects that differ only in `seed` play
  different variants and detunes, and one project's stream replays itself field for field.
- **B.** `Stinger::duck_s` is a hold: the `Music` bus stays at the stinger's depth for `duck_s`, then releases over the
  crate's `DEFAULT_FADE_S` (0.12 s).

**Against the sources.** Both rest on the crate's own documented rule for a duck, which is Gregory §14.6.3's shape and
is pinned in `src/model.rs::active_duck_db` and `tests/model.rs::a_duck_holds_then_recovers`: *full for `hold_s`, then a
linear recovery over `fade_s`*. The stinger was a wrong **argument** to that function, not a wrong model — the
rightmost column of the table below is the same function with the two arguments swapped.

For A, the source is the crate's own promise in `SirenSettings::seed`'s doc and case 17 of `cues.yaml`. No literature was
added this iteration because the literature server was unavailable for all 23 attempts (recorded in `research.md`), and
the claim does not need one: it is a wiring bug against a rule the crate already states.

| measurement | before | after | expected |
|---|---|---|---|
| `seed: 7` vs `seed: 99`, 8 plays each | **identical** streams (variant, gain, pitch) | different from the first play | two seeds are two streams |
| `seed: 7` twice, two processes | identical | identical | reproducible |
| `Music` halfway through a 0.6 s, −9 dB stinger | **−5.01 dB** | **−8.99 dB** | still held at −9 dB |
| `Music` at hold + release + a frame | 0 dB | 0 dB | recovered |
| `Music` under the dialogue duck (case 6) | −7.96 dB, recovered 1.97 s | unchanged | unchanged |
| every rendered cue | — | **byte-identical** to the before tree | no cue plays differently |

**What the halfway number means.** −5.01 dB at the midpoint of a 9 dB duck is not a small error: the music is louder
than the effects bus's own default duck (−4 dB) at the moment the stinger is meant to own the mix. On the plot
`plots/stinger-envelope-zoom.svg` it is the red curve already climbing before the blue has reached its floor.

**Verdict: GREAT.** Both defects are fixed at one call site each, both have a test that fails on the unfixed tree
(`a_projects_seed_reaches_the_stream`, `a_stinger_holds_the_music_back_for_its_whole_length`), and neither changes a
single rendered sample.

## Pass 2 — zoom

Read: `plots/stinger-envelope.svg`, `plots/stinger-envelope-zoom.svg`, `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack,tail}.png`,
and the per-frame CSV of both envelopes.

**Defects found in my own plot, and fixed before the verdict:**

1. **The blue curve read as a slope, not a shelf.** At 900 px for 60 frames, 3 frames of hold is 40 px, and the release
   is a steep line that dominated the eye. Checking the emitted polyline frame by frame is what settled it — frames
   39–48 are `y = 268.0`, i.e. **exactly −9.00 dB for 9 frames** — and the fixed defect is the zoom file, which shows
   the shelf as a shelf (`viewBox="380 30 340 250"`, 2× the linear scale).
2. **The "duck fires" label sat on the axis** and crossed the release curve. Moved above the plot's top edge, where no
   curve can reach (`examples/render.rs`).
3. **The first plot had no legend**, so a reader could not name either line. Both entries now carry a swatch and a
   label, laid out from the left, so neither can fall outside the viewBox.

**Numbers cross-checked against the data, not the raster:**

| frame | t | hold reading | fade reading |
|---|---|---|---|
| 30 | 1.00 s | −4.38 dB | −4.38 dB |
| 36 | 1.20 s | −8.92 dB | −6.43 dB |
| 42 | 1.40 s | −9.00 dB | −3.53 dB |
| 48 | 1.60 s | −9.00 dB | −0.53 dB |
| 51 | 1.70 s | −3.78 dB (releasing) | −0.07 dB |
| 59 | 1.97 s | −0.02 dB | 0.00 dB |

The two curves are identical until the hold ends, which is what a shared `active_duck_db` and a shared `approach`
should give; they part exactly where the argument differs. The blue curve's floor is as wide as the stinger.

**The cue spectrograms** are unchanged from iteration 1 and 2 (`ui-cancel-spectrum.png`: the attack inside 30 ms, a
decay to −80 dB by 0.31 s, no vertical line, no gap; `ui-cancel-attack.png`: a clean onset across the band, no click at
the 0.035 s cutoff). Loudness is unchanged: cue −24.4 LUFS / −12.1 dBFS, `mix.wav` −4.3 LUFS / +0.2 dBFS.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The seed tests are run at two seeds and three play counts (4, 8, 8), and a fresh app is
built to compare against, so the "restart" claim is checked against a *different process's* stream rather than against
the same app's earlier output. The stinger test runs at 60 fps over 1.5 s and reads the mix at three moments — fire,
mid-hold, after release — rather than trusting one sample.

**Can a developer understand it without the code?** For B, the plot answers it: two labelled curves, a dashed "duck
fires" line, and a caption naming the two readings. For A, the `Played` streams are the artifact (`render.txt` is
unchanged because the harness does not seed a table; the two test streams are the evidence), and the doc on
`SirenSettings::seed` now says what actually happens.

**What a fresh reader would still ask, and the honest answer:** why a *seed change* is not in `SirenLog`. The answer is
in the code's comment — the log is for what happened to a *sound*, and a seed is configuration — but a person watching
the Cauldron's panel will change `seed` in the file and see the stream change with no log line. That is a defensible
choice, not an oversight, and it is written down.

**One thing this pass found and did not fix:** `MusicLayers::fade_s` is still stored and never read (iteration 2's
item 3), so a project asking for an 800 ms crossfade gets 50 ms. It is next iteration's first item, it needs no decision
file, and pretending otherwise would be the kind of quiet gap this loop exists to name.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a plot label), and none is outstanding.
