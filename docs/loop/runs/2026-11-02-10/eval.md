# eval.md — baseline and the `cues.yaml` table, iteration 10

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 63 + 4 doctests green |
| `cargo test --features serde` | 70 + 4 green |
| `cargo test --features mixer` | 70 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 734 MB free at the start; `ld` failed with `errno=28` mid-run, and 15 GB was recovered by deleting
build directories this loop created (`/tmp/seedprobe`, `/tmp/siren-pub`, other spent probes). Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 28 cases green. Case 5 asserts a cue's gain and bus with the **default** table; no case swaps the table, and no case
uses `buses: []`.

## The gap, and the pick

Probed:

```
four default buses:      db = [Ambience, Dialogue, Effects, Music]
after swapping to two:   db = [Ambience, Dialogue, Effects, Music]   ← ghosts, at their old levels
                         bus_db("Ambience") = -3.3 dB
                         deepest_duck()     = Some(("Ambience", -12.0))   forever

buses: [], a cue on Effects -> played, no refusal, mixed at -80.0 dB
```

Two halves of one disagreement about **which buses exist**: the mix maps never drop a bus that left the table, and the
mix loop reads `settings.buses` directly while `bus_names()` — which `answer` and the panel use — promises the crate's
four for an empty list. The second half is rank 1 outright: an accepted play, silently at −80 dB, on the crate's own
default configuration.

Ranked:

1. **`buses: []` accepts a cue and mixes it at −80 dB** — rank 1: wrong output, silent, no refusal.
2. **`SirenMix` keeps buses that left the table** — the same disagreement, readable rather than audible; `deepest_duck()`
   names a phantom.
3. The five decision files — all public-enum questions for the owner.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a bus that leaves the table leaves the mix** | `db` has the table's buses; `bus_db` is `SILENT_DB`; `deepest_duck()` is `None` | green (fails on the unfixed tree) |
| **an empty bus list means the crate's four** | four rows, each at 0 dB; a cue on `Effects` mixes at 0 dB | green (fails on the unfixed tree) |
| **a bus that stays still eases** | one frame is part of the way, not all of it | green |
