# Should a cue with an exact pitch range spend a draw from the shared stream?

**Status: decided and built in iteration 122 — option B, and the same rule in the pickers.** Found in iteration 60 by
probing; fixed in the window where the stream was already moving (iteration 121's seed scramble had not shipped), so a
project is re-rolled **once** rather than twice. The measurement, the sources and the verdict are below; what is left for
the release is its `CHANGELOG.md` note, which is written.

## What the crate did

`answer` computed the cue's detune unconditionally:

```rust
let mut pitch = cue.pitch.at(choices.rng.next_f32());   // always drew
if let Some(extra) = request.pitch { pitch += extra; }
pitch = crate::model::played_semitones(finite_or(pitch, 0.0));
```

`PitchRange::exact(n)` is `{ low: n, high: n }`, and `at` is `low + (high - low) * unit` — with `low == high` the unit
cannot change the answer, and with `low > high` (`InvertedPitch`) `at` answers `0.0` for every unit. So the draw was
**provably useless**, and it was spent from `SirenChoices::rng`, the one stream every cue shares. The picker had the same
shape: `pick_from` and `pick` drew even when the avoided window left **one** candidate — the second play of any
two-variant cue under the default `avoid: 1`.

**Measured** (iteration 122, the real play path, the default seed): eight plays of a one-sound exact-pitch cue — a burst
weapon, a UI click — moved a three-variant cue's next six sounds from `b b c a c b` to `c b a b a a`; eight alternating
footsteps moved it to `a a b c b c`; one footstep and eight footsteps gave different observer sequences. Full output:
`runs/2026-11-04-122/evidence/before/draws-on-the-unchanged-tree.txt`.

## Why this matters

`SirenSettings::seed` is the project's reproducibility lever (`docs/guide/cues.md`: *"Two projects that differ only in
`seed` play different variants"*, and the determinism test asserts the same seed replays field for field). A draw a cue
cannot use means **adding or removing a cue from a scene changes what every other cue plays** — a project's soundscape
is a function of its seed and its **play order**, including plays that had nothing to decide. That is exactly the
coupling a seeded stream exists to remove.

## The options

### A — keep it; say so

- **Cost:** none; the stream stays as it was.
- **Benefit:** every existing project's stream unchanged, byte for byte.
- **What it costs a reader:** the guide must say that a play draws for its detune **whether or not** its range is exact,
  so "add a cue to the scene" is documented as shifting the stream. **It was documented** (iteration 60 – 121), and the
  documentation is a statement that a seeded stream is not a function of its seed.

### B — skip the draw when the number cannot change the answer ✅ **chosen**

```rust
let mut pitch = if cue.pitch.is_inverted() || cue.pitch.low == cue.pitch.high {
    cue.pitch.at(0.0)                       // the value `at` always gave this cue
} else {
    cue.pitch.at(choices.rng.next_f32())
};
```

and, in both pickers, the draw only when `choices.len() > 1`.

- **Cost:** a **behaviour change** — every project's subsequent variants move once, because the stream is the same seed
  with a shorter sequence of draws. In a shipped game that is an audible difference on the next build for no authoring
  change: **a release note, not a patch.**
- **Benefit:** the stream depends only on the plays that needed a number, so a scene's soundscape is a function of its
  seed and its decisions.
- **Why now:** iteration 121's mixer change already re-rolls every stream and **has not shipped**. Shipping both in one
  minor re-rolls a project once; shipping this later re-rolls it again. The crate is 0.1.0 and the `CHANGELOG.md`
  entries say so.
- **Why `at(0.0)` and not `low`:** `at` answers `0.0` for an inverted range — what a play of that cue always heard —
  and `low` would be a different sound (mutation M6: +2 semitones). One function, not a second rule that can disagree.

### C — a separate stream for pitch and for variants

- **Cost:** a second `Rng` on `SirenChoices` (a public field), and every project's stream changes once — more than B.
- **Benefit:** the two uses cannot perturb each other at all. **Not built:** it decouples more than the measured defect
  needs, and a second stream is a second thing a project has to reason about when it replays a capture.

## The sources

- **L'Ecuyer, Simard, Chen and Kelton**, *An Object-Oriented Random-Number Package with Many Long Streams and Substreams*,
  Operations Research 50(6):1073–1075, 2002, DOI 10.1287/opre.50.6.1073.358 — streams exist *"to facilitate synchronization
  for variance-reduction purposes, and for making independent replications"*: a use that is not a decision takes nothing.
- **Clark**, *Use of common random numbers in comparing alternatives*, WSC 1990, DOI 10.1109/wsc.1990.129543 — two runs
  compared must share the numbers of the *same* decisions.
- **Chen**, *Some insights of using common random numbers in selection procedures*, Discrete Event Dynamic Systems 22, 2012,
  DOI 10.1007/s10626-012-0142-2.
- **Kämpke**, *Multiple use of random numbers in discrete-event simulation*, Math. and Computers in Simulation 31(3), 1989,
  DOI 10.1016/0378-4754(89)90156-0 — several uses of one sequence must be accounted for.
- **Hill, Mazel, Passerat-Palmbach and Traoré**, *Distribution of random streams for simulation practitioners*,
  Concurrency and Computation 24(18), 2012, DOI 10.1002/cpe.2942.
- The crate's own precedent: iteration 51 made the picker's window the *author's*, not the allocator's, because a doc
  promised a rule the code did not apply.

## What shipped, and how it is held

| test | what it holds |
|---|---|
| `tests/play.rs::an_exact_pitch_range_leaves_the_stream_alone` | eight plays of an exact-pitch cue, and of an inverted-pitch cue, leave a `Shuffle` cue's next six sounds **and pitches** exactly where the seed put them; the inverted cue plays at **0.0** semitones, not at `low` |
| `tests/model.rs::a_forced_pick_is_not_a_draw` | `pick_from` and `pick` take nothing from the stream when one candidate is left (a one-sound cue; the second play of an alternating two-variant cue) |
| `tests/model.rs::a_pick_with_two_candidates_still_draws` | the refusal that must **not** happen: a three-variant cue under `avoid: 1` has two candidates every play and still draws, on both methods |

Six mutations (`runs/2026-11-04-122/evidence/before/new-tests-and-mutations.txt`); two of them were invisible on their
first run and the tests were widened to hold the path each one took.

## What is still open

- **The release itself**: it needs the explicit argument, and the two stream-moving changes ship in the same minor.
- A **stream-version setting** for a capture replayed across builds: still unasked for, still unbuilt.
