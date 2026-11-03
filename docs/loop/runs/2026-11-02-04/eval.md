# eval.md — baseline and the `cues.yaml` table, iteration 4

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 44 + 4 doctests green |
| `cargo test --features serde` | 51 + 4 green |
| `cargo test --features mixer` | 48 + 4 green |
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
| 7–8 — cue polyphony, bus cap in one frame | `tests/mix.rs` | green |
| 9 — an unstealable voice refuses | `tests/mix.rs` | green |
| 10–13 — each refusal | five `refusal-*.txt` | green |
| 14 — distance decides the gain | 1.000 / 0.500 / 0.250 at 4 / 8 / 16 m | green |
| 15 — a bar at tempo | 1.50 s at 160 bpm | green |
| 16 — a stinger ducks the music | `tests/music.rs` | green |
| 17 — the same seed replays; two seeds differ | `tests/play.rs` (iteration 3) | green |
| 18 — `problems()` names each mistake | `tests/mix.rs`, `tests/table.rs` | green |
| 19 — the mix does not slow a case | 8: 0.019, 64: 0.103, 256: 0.684 µs/frame | green |
| 20–21 — an ended voice frees its slot | `tests/mixer.rs` | green |
| 22 — a stinger's duck holds | `tests/music.rs` | green |
| **a layer takes the fade it was given** | 0.3 s into an 0.8 s fade: **−9.13 dB** — settled | **RED** |

## The red case, and the pick

`MusicLayers::fade_s` is accepted, stored in `Pending`, and read by nothing: `move_layers` eases every layer at
`MIX_TAU_S`. An authored 800 ms crossfade arrives in ~150 ms, and nothing says so. It is the only red case the table
has, it is the top of iteration 3's next-5, and it is a **silent** discard of a number a project supplied — the same
class as iterations 2 and 3.

Ranked:

1. **A layer ignores its authored fade** — rank 2 in the loop's order (music transitions and fades) but rank 1 in
   practice: public field, documented meaning, silently unused. **The pick.**
2. **A `VoiceLimit` refusal names the bus when the table's budget failed** — rank 3, and the fix changes a public
   enum's payload shape, so it wants a decision file; the whole item waits. Carved off.
3. **`Completes` can never be true** — already `needs-decision/remove-completes.md`; waits on the owner.
4. **A project-chosen stinger release** — `needs-decision/stinger-release.md`; shipped at `DEFAULT_FADE_S`.
5. **`Rng::next_f32`'s deliberate 24-bit truncation is undocumented** — polish; a sentence in `model.rs`.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; all 16 cue/mix/voices WAVs byte-identical | green |
| **a layer takes the fade it was given** | 0.3 s in: **−13.6 dB**; by 0.85 s: −9.07 dB | green (`tests/music.rs`) |
| **`fade_s(0)` keeps the default easing** | settles in ~150 ms as before | green |
| **a layer not named keeps its fade** | long fade still −21 dB at 0.2 s while the other is nearly there | green |
| **the derivation** | arrives within one part in a thousand at `fade_s`, and not at `fade_s / 2` | green (`tests/model.rs`) |
