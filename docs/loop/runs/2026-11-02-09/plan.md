# plan.md — iteration 9: a duck waits for a sound that plays

## The pick, and why it beats the runner-up

**Pick:** `play_stingers` arms its duck and logs `Ducked` **before** `play_requests` answers, so a refused stinger
leaves the `Music` bus dipped −9 dB for a sound that never happens, and the log states both facts on adjacent lines.

**Runner-up:** `unwrap_or("Music")` lets a cue **not in the table** pass the bus check. It is the same bug's easiest
cause and is fixed by the same change.

The pick wins on the loop's first criterion at its sharpest: a duck that corresponds to no sound, plus the log
contradicting itself. The remaining next-5 items are decision-blocked.

**Carved off:** a stinger that queues until the budget frees a voice, or that retries. Refusing and leaving the music
alone is the honest answer; queueing is a feature.

## Design

`src/music.rs` (and only it, beyond a `CHANGELOG`/guide line):

```rust
/// A stinger whose play has been written and not yet answered.
struct PendingStinger { cue: String, duck_db: f32, duck_s: f32 }
```

- `MusicState` gains `pending_stingers: Vec<PendingStinger>` (private; `MusicState` is a public `Resource` but its
  fields are private apart from `score`).
- `play_stingers`:
  - a cue **not in the table** → `Entry::Refused` naming it, `continue`. This removes `unwrap_or("Music")`.
  - a cue on another bus → refused as today.
  - otherwise push a `PendingStinger` and write the `Play`. **No duck, no `Ducked` line.**
- `arm_stinger_ducks` (new), `.after(SirenSystems::Play).before(SirenSystems::Mix)`:
  - for each `Played`, find its pending stinger by cue, arm `clock.manual("Music", …)`, log `Ducked`, remove it.
  - drop pending entries whose cue was refused this frame — so a refused stinger leaves nothing.
  - clear the list at the end of the system: a stinger's answer is always in the frame it was written, or never.

## Every field touched on a serde type

None. `MusicState` is not a serde type (only `MusicScore`, `CueTable`, `SirenSettings` are), and no cue/mix field moves.

## Tests that will prove it

`tests/music.rs`:

1. `a_stinger_whose_cue_is_unknown_ducks_nothing` — the probe as a test: `Music` stays at 0 dB, one `Refused`
   (`UnknownCue`), **no** `Ducked` line. Fails on the unfixed tree (ducked −9.0).
2. `a_stinger_whose_sound_is_missing_ducks_nothing` — the second cause: cue on `Music`, no handle, so `MissingSound`;
   the bus is untouched and the log's only line about it is the refusal. Fails on the unfixed tree.
3. `a_stinger_that_plays_ducks_on_the_frame_it_fires` — the case that must not regress: a real cue, `Music` at −9.0 dB
   on the same frame, and a `Ducked` whose `by` is `"stinger"`.
4. `a_stinger_on_another_bus_is_still_refused` — the existing rule, unchanged, with no duck.

`tests/mix.rs` gains the ordering assertion inside test 3: the duck must be visible to the **same frame's** `update_mix`,
which is what `before(SirenSystems::Mix)` buys.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 16, the stinger's duck, must stay green and is the
  non-regression case);
- `cargo run --release --example perfcheck` before/after — a `Vec` scan per stinger request, bounded by the number of
  stingers in a frame (one);
- `examples/render` before/after: cue WAVs byte-identical; a new `refusal-stinger-*.txt` for the unknown cue, since a
  refusal is a decision and the harness already writes those.

## Risk

`arm_stinger_ducks` must run before `update_mix` or the duck lands a frame late, which is audible as a one-frame gap
between the hit and the music stepping back — 16 ms at 60 fps, and the crate already went to some trouble in iteration
3 to make the hold land on the frame it fires. The `.before(SirenSystems::Mix)` is not decoration.

The pending list is frame-local by construction (cleared each run), so a stinger that is neither played nor refused —
impossible today, since every `Play` is answered — cannot leak an entry.
