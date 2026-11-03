# plan.md — iteration 13: the cap is a play-time budget, and the docs say so

## The pick, and why it beats the runner-up

**Pick:** `Cue::max_instances`'s doc reads *"How many of this cue may sound at once"* — a standing promise. Probed, a
table edited under live voices leaves two sounding against a cap of one, and the crate's R4 and Gregory §14.5.8.4 both
describe a **request-time** cap. Both readings are defensible; only one is written down, and the written one is the
one the crate does not implement.

**Runner-up:** the five decision files — public-enum questions for the owner.

The pick wins because it is the only unblocked item left, it is a genuine contract gap a project hits by editing a
table at runtime (the ordinary path — `CueTable` is documented as a resource a game may insert), and its repair costs
no behaviour change.

**Carved off:** reconciling live voices against a changed table. That would despawn a voice the game did not stop, on a
frame it did not choose, and log a stop it did not request — three surprises to replace one — and it needs rules for
which voice goes and for a table edited twice in a frame. Named in the log's next-5 as a feature question.

## Design

`src/cue.rs`, `src/music.rs` (one sentence each) and `docs/guide/cues.md`:

- `Cue::max_instances`'s doc gains: **"a play-time budget, not a standing invariant"**, with the reason — a table
  edited while voices sound does not end them, because ending them would cut a sound on a frame the game asked for
  nothing.
- `CueTable::insert`'s doc (the path a project uses to change one at runtime) points at the same sentence.
- The guide's voice-budget section says it in a project's words.

No `src/` behaviour changes.

## Every field touched on a serde type

None. One doc comment on `Cue::max_instances`, one on `CueTable::insert`.

## Tests that will prove it

`tests/mix.rs`:

1. `a_cap_lowered_under_live_voices_does_not_end_them` — the probe as a test: two voices at `max_instances: 2`, the table
   edited to 1, a third play refused `VoiceLimit`, **both voices still live**, and no `Stopped` line for either. This
   pins the documented reading so a later change cannot silently adopt the other one.
2. `a_cap_raised_under_live_voices_lets_the_next_play_through` — the pair that stops the first from passing vacuously:
   with the table raised from 1 to 2 while one voice is live, the next play is **answered `Played`**.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 7 is the polyphony case and must stay green);
- `cargo run --release --example perfcheck` before/after: no code change, so the figure is the machine's and is reported
  as such;
- `examples/render` and `examples/music`: cue WAVs byte-identical, music transitions still on the bar;
- the probe that found it, run against the fixed tree, with the doc's sentence beside it.

## Risk

A documentation-only iteration risks being a pass that ships nothing. This one ships the **answer** to a question the
crate's own field posed and left open, plus two tests that hold it — and it records, as a measurement, that five other
candidates in the handle class were safe. That is smaller than iteration 12 and is reported as such.
