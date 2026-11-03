# plan.md — iteration 16: a layer must name a cue that loops

## The pick, and why it beats the runner-up

**Pick:** a `MusicScore` layer naming a `Mode::Once` cue is **started anyway**, and iteration 15's restart rule then
retriggers it **every frame** (10 plays in 10 frames, no refusal, no log line), while `MusicScore::problems` already
states the rule and an **unknown** cue is properly refused. A rank-1 wrong-output bug: the crate says a thing is
impossible and then does it at frame rate.

Beats the runner-up (`SirenChoices` reset semantics): probing that showed the seeded stream is **sound** — a fresh
`SirenChoices::new(seed)` reproduces the stream, re-writing the same seed continues it, and resetting the public `rng`
alone reproduces it too. Nothing to fix.

## The design

One rule, in `start_missing` (`src/music.rs`), where the play is decided:

1. **A layer whose cue is not `Mode::Loop` is not started.** `start_missing` gains `&CueTable`. The check reads the
   cue the layer names **this frame**, so a table edit that turns a layer's cue into a one-shot stops its restart and a
   table edit that turns it back starts it.
2. **One log line per layer, not per frame.** `LiveLayer` gains `noted: bool`; the line is written the first time the
   layer is refused-for-being-a-one-shot, in `problems()`' own words
   (*"names cue `hit`, which is not a loop, so it cannot hold a layer"*), and not again.
3. **The layer stays in `names()` and in the score.** A bad cue is a bad cue; dropping the layer would hide the mistake
   a second way.

**A missing cue already has a path**: `Play` is refused with `Refusal::UnknownCue` and the log line is written by
`play_requests`. For the one-shot case a `Refusal` variant would be the symmetric answer, and it is **public API**:
`needs-decision/not-a-loop-refusal.md`. This iteration ships the enforcement and the line; the variant waits.

## Fields

No new field on `CueTable`, `SirenSettings` or `MusicScore`. `LiveLayer` (private) gains `noted: bool`, defaulting
`false`.

## Tests

- `a_layer_whose_cue_is_not_a_loop_is_not_started` — a `Mode::Once` layer beside a loop layer: the loop plays, the
  one-shot does not, and exactly one log line names both the layer and the cue.
- `a_not_a_loop_layer_is_noted_once` — 200 frames: still exactly one line.
- `a_layer_whose_cue_becomes_a_loop_starts` — the table is edited to `Mode::Loop`; the layer starts on the next frame.
- The existing 21 music tests must stay green, and every rendered cue byte-identical.

## The carve

- **Not** the `Refusal::NotALoop` variant: public API, decision file.
- **Not** a `MusicScore::problems` change: it already has the sentence.
