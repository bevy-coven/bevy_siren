# eval.md — baseline and the `cues.yaml` table, iteration 5

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 48 + 4 doctests green |
| `cargo test --features serde` | 55 + 4 green |
| `cargo test --features mixer` | 52 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 2.1 GB free, 100 % used. Open issues: none.

## The `cues.yaml` table against the crate as it stands

| Case | Observed | Green? |
|---|---|---|
| 1–4 — variants, pitch | `tests/play.rs`, `tests/model.rs` | green |
| 5 — a cue's gain and bus | `cue-ui-cancel-0: -6.00 dB` | green |
| 6 — a duck holds then recovers | deepest −7.96 dB, recovered 1.97 s | green |
| 7 — a cue's own polyphony steals | `tests/mix.rs` | green |
| 8 — a bus cap holds in one frame | `tests/mix.rs` | green |
| 9 — an unstealable voice refuses | `tests/mix.rs` | **green, but the payload is wrong** (below) |
| 10–13 — each refusal | five `refusal-*.txt` | green |
| 14 — distance decides the gain | 1.000 / 0.500 / 0.250 at 4 / 8 / 16 m | green |
| 15 — a bar at tempo | 1.50 s at 160 bpm | green |
| 16 — a stinger ducks the music | `tests/music.rs` | green |
| 17 — a seed replays; two seeds differ | `tests/play.rs` | green |
| 18 — `problems()` names each mistake | `tests/mix.rs`, `tests/table.rs` | green |
| 19 — the mix does not slow a case | 0.022 / 0.096 / 0.667 µs/frame | green |
| 20–21 — an ended voice frees its slot | `tests/mixer.rs` | green |
| 22 — a stinger's duck holds | `tests/music.rs` | green |
| 23 — a layer takes its fade | `tests/music.rs` | green |

## The gap the table has, and the pick

Cases 7–9 each assert `matches!(reason, Refusal::VoiceLimit { .. })` — the wildcard is the hole. A refusal can be the
right *kind* and the wrong *content*, and the table had no case that reads the content.

A probe with the real ECS proved both non-bus caps reported a bus:

| cap that refused | sentence |
|---|---|
| `max_voices: 1` (the table), two buses with room | ``no voice free on bus `Ambience` `` |
| the cue's `max_instances: 1`, its bus at 0 of 8 | ``no voice free on bus `Effects` `` |

The first names a bus the cue does not play on. Rank 1 — a refusal that is confidently wrong is worse than a silent
one, because it sends a reader somewhere specific.

Ranked:

1. **A refusal names a bus when no bus refused** — wrong output, silent in the sense that nothing crashes and no test
   fails. **The pick.**
2. **`start_pending` reports a decode failure as `MissingSound { sound: voice.cue }`** — the cue's name in a field
   documented as a *sound key*. Found by this iteration's own probe; the same class, next item.
3. **`Completes` can never be true** — `needs-decision/remove-completes.md`.
4. **A project-chosen stinger release** — `needs-decision/stinger-release.md`.
5. **`Rng::next_f32`'s 24-bit truncation is undocumented** — polish.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a full table names no bus** | `VoiceLimit { bus: None }`, `"no voice free"` | green (`a_full_table_budget_names_no_bus`) |
| **a cue at its polyphony names no bus** | `VoiceLimit { bus: None }` | green (`a_cue_at_its_own_polyphony_names_no_bus`) |
| **a full bus still names it** | `VoiceLimit { bus: Some("Effects") }` | green (`a_full_bus_names_that_bus`) |
| **payload and sentence match** | each shape has one line | green (`every_voice_limit_sentence_matches_its_payload`) |
