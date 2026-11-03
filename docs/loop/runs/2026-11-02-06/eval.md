# eval.md — baseline and the `cues.yaml` table, iteration 6

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 52 + 4 doctests green |
| `cargo test --features serde` | 59 + 4 green |
| `cargo test --features mixer` | 56 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 2.0 GB free, 100 % used. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 24 cases green — 1–4 variants and pitch, 5 gain and bus, 6 the dialogue duck, 7–8 the caps, 9 the unstealable
refusal, 10–13 the four refusals, 14 the distance law, 15 the bar, 16 the stinger, 17 the seed, 18 `problems()`, 19
the frame cost, 20–21 the retired voice, 22 the stinger's hold, 23 the layer fade, 24 the refusal's bus.

## The gap the table has, and the pick

Case 11 asserts `Refusal::MissingSound { sound }` where `sound` is a **key with no handle** — the `answer` path. No case
covers the *other* producer of that variant: `start_pending`, when the decoder refuses bytes. A probe proved it names
the **cue**:

```
no handle:     MissingSound { sound: "step_03" }        -> no handle for sound `step_03`
undecodable:   MissingSound { sound: "footstep.grass" } -> no handle for sound `footstep.grass`
```

Two causes of one variant, two different subjects, and the second one names a file that never existed.

Ranked:

1. **A decode refusal names the cue instead of the sound key** — rank 1: a refusal that is confidently wrong, telling
   a project to look for `footstep.grass` in `audio/`. **The pick.**
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`; waits on the owner.
3. **A variant that distinguishes *undecodable* from *never mapped*** — filed this iteration as
   `needs-decision/undecodable-sound.md`.
4. **`Completes` can never be true** — `needs-decision/remove-completes.md`.
5. **`Rng::next_f32`'s 24-bit truncation is undocumented** — polish.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a decode refusal names the key** | `MissingSound { sound: "step_03" }` | green (`tests/mixer.rs::a_sample_the_decoder_refuses_names_its_sound_key`) |
| **a missing handle still names the key** | unchanged, `step_03` | green (`a_key_with_no_handle_still_names_the_key`) |
| **the sentence covers both causes** | `no handle or undecodable bytes for sound \`step_03\`` | green (`tests/play.rs`) |
