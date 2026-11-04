# Iteration 123 — plan

## The item

`needs-decision/log-cursor.md` option **4**: **the crate prints an authoring refusal once on the console**, with
`target: "bevy_siren"`, through `bevy_log` (already a dependency) — so it reaches the terminal a game's own subscriber
prints, and a game can silence it with `RUST_LOG=bevy_siren=off`. `VoiceLimit`, `SpatiallyMute` and every play's log
entry are unchanged.

Why it beats the runner-up (a moving owner's level): a **silent** refusal is the loop's own "most valuable bug" class —
a cue that never plays and never says why is the hardest thing in a game to find — and this is a measured gap, not a
design question: the crate has the sentence, writes it to a ring buffer, and prints nothing. The release on top of the
top 5 needs the owner's explicit argument, so it is not this iteration's.

## The measurement first, as the decision file asks

`examples/scratch_refusals.rs` (deleted after the evidence is kept) plays, through the real plugin and the real play
path with no device:

| case | plays | expected on the console |
|---|---|---|
| a healthy footstep (two variants, both handles) | 8 | **0** |
| `footstep.grss` — not in the table | 100 in one frame | **1** after the change; 0 before |
| `footstep.snow`, a second unknown cue, same frame | 1 | **1** (a second cue is a second mistake) |
| the same unknown cue over 8 frames | 8 | **8** (one per frame — it is the same mistake repeating, and a frame is a second of console at 60 fps) |
| a one-sound cue with no handle | 1 | **1** (`MissingSound`) |
| a cue on `Reverb` | 1 | **1** (`UnknownBus`) |
| a cue with no sounds | 1 | **1** (`NoSounds`) |
| a budget of 2, 40 plays | 40 | **0** (`VoiceLimit` is normal operation) |
| `Play::at` with no listener | 1 | **0** (`SpatiallyMute` is architectural) |

The before-run on the unchanged tree prints **0** for every row: that is P1, and it is the defect.

## The design

In `src/play.rs`'s refusal arm, after the log entry is pushed and the `Refused` message written:

```rust
if reason.is_authoring() {
    warn!(target: "bevy_siren", "refused `{cue}`: {sentence}", cue = request.cue, sentence = ...);
}
```

- **`Refusal::is_authoring()`** is a new **private** helper in `play.rs` (no public API): `true` for `UnknownCue`,
  `NoSounds`, `MissingSound` and `UnknownBus`; `false` for `VoiceLimit` and `SpatiallyMute`. The split is written where
  the refusals are, with the reason for each.
- **The sentence is the log's own**, from `refusal_sentence` (so the console and the panel cannot drift), prefixed with
  the cue it was asked for — the log entry has no cue outside its own `Entry::Refused { cue, reason }`, and a console line
  without the cue name is half an answer.
- **`target: "bevy_siren"`** makes the line filterable by target (`RUST_LOG=bevy_siren=off`), and it is a `warn!` because
  an authoring refusal is a fault, not chatter. `bevy_log` is in `Cargo.toml` already (`Cargo.toml:120` says why).
- **No new field, method, message, schema or dependency.** `SirenSettings` is untouched; a setting would be a schema the
  editor reads, and `RUST_LOG` is the setting a Rust project already has.

## The one thing to check before building

**A headless test with no subscriber** (every existing test) must not panic or print: `tracing` events with no
subscriber are dropped, which is what makes this safe for the 600 existing tests. That is checked by running the suite,
not assumed.

## Modules touched

- `src/play.rs` — `is_authoring`, the `warn!`, and the doc of `Refused`/`Refusal` saying where a line goes.
- `src/log.rs` — the header sentence *"Nothing here is a `println!`. A crate that prints is a crate a game cannot
  silence"* is **half true now** and is rewritten: the crate does print an authoring refusal, filterably; the log is
  still written in silence.
- `docs/guide/cues.md` — the refusal section gains the console rule and the `RUST_LOG` line.

## The tests that will prove it

| test | where | what it holds |
|---|---|---|
| `an_authoring_refusal_is_printed_once_with_the_cues_name` | `tests/all.rs`'s new `tests/console.rs` | the target, the sentence, the cue; a hundred plays in one frame are **one** line |
| `a_budget_refusal_is_not_printed` | same | 40 plays against a cap of 2 print **0** lines, and `VoiceLimit` is refused |
| `a_spatial_refusal_is_not_printed` | same | `Play::at` with no listener prints nothing |
| `the_console_and_the_log_say_the_same_thing` | same | the line's sentence is the `Entry::Refused` reason, byte for byte |
| `an_app_with_no_subscriber_is_silent` | same | the ordinary headless app runs 100 plays and does not panic |
| `a_key_without_its_extension_is_named_on_the_console` (existing, `tests/quick_start.rs`) | — | still green: the quick start's own `report` prints the log, and now the crate prints one line too — **the existing test filters by its own module target**, so it is unaffected; that is checked, not assumed |
| the whole `cues.yaml` table | — | unchanged |

A capture at the end: the real `examples/quick_start` run with the old keys on this Mac's console, and the same for a
correct table (no line), as the picture.

## Predictions

`research.md` has P1 – P9, written before any code.

## Carried off

- A `SirenSettings` switch for printing (a schema change; `RUST_LOG` does it).
- Printing `VoiceLimit` and `SpatiallyMute` (normal operation and an architectural state).
- The release, a moving owner's level, score layers starting together: unchanged in the log's top 5.
