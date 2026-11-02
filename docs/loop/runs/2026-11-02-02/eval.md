# eval.md — baseline and the `cues.yaml` table, iteration 2

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 41 + 4 doctests green |
| `cargo test --features serde` | 48 + 4 green |
| `cargo test --features mixer` | 41 + 4 green (no mixer-specific test existed) |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green, one linker warning (`__eh_frame section too large`, from the `mixer` example's Bevy link, not from the crate) |

**Nothing was red at the baseline, and that was the problem.** The suite was green and the `mixer` path was broken,
because no test in the crate ever gave a voice a `MixerVoice` and then let its sample end.

**Machine state at the baseline measurement:** load average 8.08 / 8.50 / 6.05 on this 8-core Mac; disk 99 % full
(5.7 GB free). Both numbers belong with the `perf.md` figures.

## The `cues.yaml` table against the crate as it stands

| Case | Observed | Green? |
|---|---|---|
| 1–4 — variants, pitch | proven by `tests/play.rs`, `tests/model.rs` | green |
| 5 — a cue's gain and bus | `cue-ui-cancel-0: -6.00 dB`; four default buses | green |
| 6 — a duck holds then recovers | `bus-Music: deepest -7.96 dB, recovered by 1.97 s` | green |
| 7 — a cue's own polyphony steals | `tests/mix.rs::a_cue_respects_its_own_polyphony` | green |
| 8 — a bus cap holds in one frame | `tests/mix.rs::three_requests_in_one_frame_fill_a_cap_of_two_with_no_third` | green |
| 9 — an unstealable voice refuses | **no test**; the rule is in `room_for` and `steal`, but nothing asserts a refusal that must happen (case 9) against one that must not (case 5) | **red: the case has no assertion** |
| 10–13 — each refusal | five `refusal-*.txt`, each the crate's own sentence | green |
| 14 — distance decides the gain | `spatial: 1.000 at 4 m, 0.500 at 8 m, 0.250 at 16 m` | green |
| 15 — a bar at tempo | `music: a bar at 160 bpm from 0.3 s is 1.50 s` | green |
| 16 — a stinger ducks the music | `tests/music.rs::a_stinger_ducks_the_music` | green |
| 17 — the same seed replays | `tests/model.rs::the_rng_repeats_its_own_sequence` pins the stream, but no test replays a **whole `Played` stream** field for field | **red: the case's observation does not exist** |
| 18 — `problems()` names each mistake | `tests/mix.rs::problems_names_each_mistake`, `tests/table.rs` | green |
| 19 — the mix does not slow a case | 8: 0.062 µs, 64: 0.353 µs, 256: 1.396 µs per frame | green |

**Case 5's own pairing rule is what found the item.** The table's note says a refusal is green only when it is the
*right* refusal, "so each is paired with a play that succeeds". Case 8/9 are about a budget that is *full*; no case
asks what happens when a budget should have **emptied**. On the `mixer` backend it never does — that is this
iteration's bug, and it is the one case the table could not see because it had no row for it.

## Proving the bug before fixing it

A throwaway crate in `/tmp/leakprobe` (path dependency on this crate, `features = ["mixer"]`, no `AudioPlugin`),
with a fake `AudioOutput` that reports its voice finished:

```
first frame: voices=1 fake_play_calls=[("Effects", 0.0)]
after end reported: voices=1  (expected 0)
log: ["0.00s played `hit` (k) on Effects at 0.0 dB"]
```

The sample has ended, the backend says so, and the crate keeps the voice and never writes `ended`. With
`max_voices: 32`, the 33rd one-shot on that backend is refused `VoiceLimit` for the life of the process. The
regression test `tests/mixer.rs::a_one_shot_voice_ends_and_frees_its_slot` reproduces exactly that and fails on the
unfixed tree (checked by reverting the two hunks: `left: 1, right: 0`).

## The table after the change

| Case | After | Green? |
|---|---|---|
| all of the above | unchanged: `render.txt` and all 17 rendered WAVs are byte-identical before and after | green |
| **new — a one-shot frees its slot** | 40 plays against a cap of 32: 40 `Played`, 0 `VoiceLimit`, ≤1 live voice, 40 `ended` lines | green (`tests/mixer.rs`) |
| **new — a layer's gain reaches the mixer voice** | a layer moved to −3 dB reads −3 dB in `Applied`, which is the number the mixer's voice stage holds | green (`tests/mixer.rs`) |
| 9 — an unstealable voice refuses | still no test | still red, next item |
| 17 — a whole `Played` stream replays | still pinning the RNG rather than the stream | still red, next item |
