# eval.md — baseline and the `cues.yaml` table, iteration 3

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 41 + 4 doctests green |
| `cargo test --features serde` | 48 + 4 green |
| `cargo test --features mixer` | 45 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 8.4 GB free, 99 % used; CI's last run on `main` was green.

## The `cues.yaml` table against the crate as it stands

| Case | Observed | Green? |
|---|---|---|
| 1–4 — variants, pitch range | `tests/play.rs`, `tests/model.rs` | green |
| 5 — a cue's gain and bus | `render.txt`: `cue-ui-cancel-0: -6.00 dB` | green |
| 6 — a duck holds then recovers | `bus-Music: deepest -7.96 dB, recovered by 1.97 s` | green |
| 7–8 — cue polyphony, bus cap in one frame | `tests/mix.rs` | green |
| 9 — an unstealable voice refuses rather than being stolen | covered by `tests/mix.rs::a_full_bus_steals_the_oldest_and_then_refuses` and `the_table_budget_refuses_rather_than_grow` | green (**was wrongly listed red in iteration 2's eval**; the cases exist and pass) |
| 10–13 — each refusal | five `refusal-*.txt`, each the crate's own sentence | green |
| 14 — distance decides the gain | `spatial: 1.000 at 4 m, 0.500 at 8 m, 0.250 at 16 m` | green |
| 15 — a bar at tempo | `music: a bar at 160 bpm from 0.3 s is 1.50 s` | green |
| 16 — a stinger ducks the music | `tests/music.rs::a_stinger_ducks_the_music` | green for the depth; **red for the shape** (see below) |
| 17 — **the same seed replays the same sounds** | two `Played` streams from two projects that each write `seed: 7` | **RED** |
| 18 — `problems()` names each mistake | `tests/mix.rs`, `tests/table.rs` | green |
| 19 — the mix does not slow a case | 8: 0.062 µs, 64: 0.353 µs, 256: 1.396 µs per frame | green |
| 20–21 — an ended voice frees its slot | `tests/mixer.rs`, `runs/2026-11-02-02/evidence/` | green |

## The four candidates from iteration 2's next-5, checked against the code

| # | Claim | Verdict |
|---|---|---|
| 1 | Case 9 has no test | **False.** `a_full_bus_steals_the_oldest_and_then_refuses` and `the_table_budget_refuses_rather_than_grow` both pin it. Iteration 2's eval was wrong; this table corrects it. |
| 2 | Case 17 pins the RNG, not the stream | **True, and worse than stated.** `tests/model.rs::the_rng_repeats_its_own_sequence` proves `Rng` repeats. But `SirenSettings::seed` is read by **nothing**: `SirenChoices` is `init_resource` with `Default`, which is `new(SirenSettings::default_seed())`, and no system ever rebuilds it from the settings. A project that ships `seed: 7` and a project that ships `seed: 99` get the *same* stream. |
| 3 | `MusicLayers::fade_s` and `Stinger::duck_s` are stored and never read | **Half true, and the half that is true is a bug.** `MusicLayers::fade_s` is stored in `Pending` and never read: every move eases at `MIX_TAU_S` regardless. `Stinger::duck_s` *is* read, but as the **fade** rather than the **hold**: `clock.manual("Music", duck_db, now, 0.0, duck_s)` puts it in the fade argument, so the stinger's duck starts recovering on the frame it fires and 1 s of `duck_s` means a 1 s linear recovery, not a 1 s hold. |
| 4 | A `VoiceLimit` refusal names the bus even when the table's budget failed | **True.** `answer` always passes `bus: Some(bus)`, so a refusal caused by `max_voices` reads "no voice free on bus `Effects`", which sends a person to the wrong row of the table. |

## Ranked, for `plan.md`

1. **`SirenSettings::seed` is dead** — a documented reproducibility promise that does not hold: a game cannot choose
   its stream, so a replay, a capture and an A/B of an audio change are not reproducible per project. Rank 1: wrong
   output, silent, and it invalidates case 17 entirely.
2. **A stinger's `duck_s` is a fade, not a hold** — the duck starts recovering on the frame the stinger fires, so the
   music comes back *under* the stinger instead of stepping back for it. Rank 1: wrong output, audible.
3. **`MusicLayers::fade_s` is ignored** — a crossfade a project asks for silently takes 50 ms. Rank 2–3: an
   authored number that does nothing.
4. **`VoiceLimit` names the wrong culprit** — rank 3, authoring ergonomics; it needs a `Refusal` shape change and so a
   decision file.
