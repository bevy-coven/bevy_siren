# Iteration 122 — research: a draw that cannot change the answer

**Item**: `docs/loop/needs-decision/exact-pitch-draw.md`, option **B** — and the same rule where the picker already
knows its answer. A play draws from the one shared stream even when the number cannot change what is heard: a cue whose
`pitch` range is exact, and a pick with only one candidate left.

## The sources

| source | what it says | where |
|---|---|---|
| L'Ecuyer, Simard, Chen and Kelton, *An Object-Oriented Random-Number Package with Many Long Streams and Substreams*, **Operations Research 50(6):1073–1075, 2002** | *"Multiple independent streams of random numbers are often required in simulation studies, for instance, to facilitate synchronization for variance-reduction purposes, and for making independent replications."* A stream exists so that one use of randomness can be *synchronised* with another — i.e. so two runs can be compared without one use's draws moving the other's. | DOI 10.1287/opre.50.6.1073.358 |
| Clark, *Use of common random numbers in comparing alternatives*, **Winter Simulation Conference 1990** | Comparing two designs must give each the *same* random numbers, or the difference measured is partly the randomness (common random numbers). | DOI 10.1109/wsc.1990.129543 |
| Chen, *Some insights of using common random numbers in selection procedures*, **Discrete Event Dynamic Systems 22, 2012** | The same: shared streams are a technique for making alternatives comparable, and they only work when the shared draws correspond to the same decisions. | DOI 10.1007/s10626-012-0142-2 |
| Kämpke, *Multiple use of random numbers in discrete-event simulation*, **Mathematics and Computers in Simulation 31(3), 1989** | Several uses of one sequence must be accounted for; a use that does not need randomness consumes a number the other uses then miss. | DOI 10.1016/0378-4754(89)90156-0 |
| Hill, Mazel, Passerat-Palmbach and Traoré, *Distribution of random streams for simulation practitioners*, **Concurrency and Computation 24(18), 2012** | A practitioner's guide to partitioning randomness into streams; the reason to partition is that unrelated uses must not perturb each other. | DOI 10.1002/cpe.2942 |
| L'Ecuyer and Leydold, *rstream: Streams of Random Numbers for Stochastic Simulation*, 2005 | Streams and **substreams** as the unit a simulation resets, so a run can be replayed from a boundary rather than from zero. | DOI 10.57938/1225aa05-185d-4a72-8eb9-14cb45b2fc10 |
| this crate, `docs/guide/cues.md` | *"Two projects that differ only in `seed` play different variants and different detunes from the first play; the same number plays the same sequence twice."* | the promise the coupling breaks |
| this crate, iteration 51 (`AGENTS.md`) | The picker window was the allocator's capacity rather than the author's: a rule that reads as one thing and behaves as another. The fix was to make the code mean what its doc says. | precedent for B over A |

The technique, in the sources' terms: **randomness is partitioned by *use*, and a use that is not a decision takes
nothing.** A cue whose pitch cannot vary, and a pick whose only remaining candidate is forced, are not decisions. Today
they take a number from the stream the decisions share, so a scene's soundscape is a function of its **play order** —
including plays that had nothing to decide — not of its seed and its decisions. That is the coupling a stream exists to
remove, and it is the same class of defect as iteration 51's: the doc promises a rule, the code applies a different one.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| streams needed for comparability | two runs that are compared must share the stream for the *same* decision | Clark 1990; Chen 2012 |
| substream granularity | a simulation resets a *substream*, never a mid-stream position | L'Ecuyer & Leydold 2005 |
| uses of one sequence that are not randomness | must not consume, or every later use shifts | Kämpke 1989 |
| this crate's draw per play (before) | **2**: one for the variant, one for the detune, whether or not either can change the sound | `src/play.rs:624`, `:661`; `src/model.rs:465`, `:501` |
| this crate's draw per play (after) | **0** for a one-variant cue with an exact pitch (a burst, a UI click); **1** for a two-variant `avoid: 1` cue's first play and **0** for every later play (it alternates) | this iteration's probe |

## What good sounds like

Nothing about the sound changes: every play of a cue that *can* vary still draws from the same stream, and the variant
and detune chosen are unchanged for every cue whose answer depended on a number. What changes is **which later plays are
affected by which earlier ones**: a burst weapon (one sample, exact pitch) fired a hundred times, a UI click, or a
two-variant footstep's alternation must leave another cue's variants exactly where the seed put them. A listener hears
the same sounds; a developer A/B-testing an audio change stops hearing the change move because they added a click.

## Predictions (written before the build; scored in `review.md`, wrong ones first)

1. **P1** — the unchanged tree: a `gun.burst` (one sound, `pitch: exact`) played **8 times** moves a later three-variant
   `Shuffle` cue's six picks, so its sequence differs from the same cue played alone; and a two-variant `avoid: 1`
   footstep played 8 times also differs. Measured through the real play path, default seed.
2. **P2** — after: the burst's eight plays leave the `Shuffle` cue's six picks **equal** to the cue played alone, and
   eight footsteps leave it equal to **one** footstep's (the first play of a two-variant alternating cue still draws,
   because it has two candidates; every later play has one).
3. **P3** — the picker's own test: `VariantPicker::pick_from` with exactly one candidate left leaves the `Rng` where it
   was (the next draw of a clone is the same as the next draw after the pick).
4. **P4** — the 21 rendered WAVs are **byte-identical** to iteration 121's: the harness renders a cue's speed at
   `pitch.at(0.0)` and its `variants …` report lines are drawn from a fresh picker per cue.
5. **P5** — the `variants …` lines in `render.txt` **do** change for cues whose sequences depended on the shared stream,
   because the render harness's per-cue pickers share one `Rng`.
6. **P6** — every existing seed test still passes: the same seed still replays the same sounds field for field, and two
   seeds still differ. (The *sequences* move — a release note — but determinism is untouched.)
7. **P7** — `perfcheck` is inside its band and the change costs nothing measurable: it removes a draw, not adds one.
8. **P8** — a cue with a **3-variant** `avoid: 1` window still draws on every play (2 candidates left, so the number
   decides) and still never repeats back to back: the fix must not turn a real choice into a forced one.
9. **P9** — an **inverted** pitch range (`low > high`, which `problems()` reports and `at` answers `0.0` for) also stops
   drawing: the unit cannot change the answer there either.

## Carried off, not built

- **A separate stream per use** (`exact-pitch-draw.md` option C): a second `Rng` for pitch, or one per cue. It would
  decouple more, and it costs a second public field and a second release note; recorded in the decision file.
- **A stream-version setting**: still no consumer has asked.
- The carried top-5 items (authoring refusals, a moving owner's level, score layers starting together, a person
  listening) stay where they were.
