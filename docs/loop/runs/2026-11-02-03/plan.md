# plan.md — iteration 3: the seed a project writes, and the hold a stinger promises

## The pick, and why it beats the runner-up

**Pick A — `SirenSettings::seed` reaches the stream.** Proved dead: `seed: 7` and `seed: 99` give byte-identical
`Played` streams. Rank 1 by the loop's own order (wrong output, silent) and the only item that invalidates a whole
`cues.yaml` case: case 17 exists to prove reproducibility and cannot, because a project cannot choose a stream.

**Pick B — a stinger's `duck_s` is a hold, not a fade.** Rank 1 on the same criterion (wrong output, audible): the
music recovers *underneath* the stinger. It is a two-line fix in one call site, and its evidence is a bus plot, which
the loop can already produce.

**Runner-up:** `MusicLayers::fade_s` is stored and never read. Real (rank 2–3), but a crossfade that takes 50 ms instead
of an authored 800 ms is a less severe error than a seed that does not exist or a punctuation mark that dissolves. It
is next.

**Carved off:** the `VoiceLimit { bus }` mislabel (rank 3, and a `Refusal` shape change that needs a decision file) and
a new `Stinger` release field (same, a public-message change). Both go to `needs-decision/`; the fix ships without them.

## Design

### A. One owner for the random stream

`src/play.rs`:

- A public constructor already exists — `SirenChoices::new(seed)`. Nothing new is needed on the type.
- `SirenChoices` gains no field. Its seed is **remembered** so a settings change can be detected: `seeded: u64`.

`src/lib.rs`:

- `seed_choices`, a new system in `SirenSystems::Choose`. It reads `Res<SirenSettings>` and `ResMut<SirenChoices>`;
  when `settings.seed != choices.seed()` it replaces the stream with `SirenChoices::new(settings.seed)` — which resets
  every variant history, because a different seed means a different sequence from the first play — and logs one
  `Entry::Seeded { at_s, seed }`.
- Ordering: `Choose` is already before `Play`, and `play_requests` reads `SirenChoices` in `Play`, so the first play
  after a seed change already uses the new stream. `start_layers` (music) also writes `Play` in `Choose`; `seed_choices`
  must run **before** it, so a score started on the same frame as a seed change draws from the new stream too.

`src/log.rs`:

- A new `Entry::Seeded { at_s: f64, seed: u64 }`. `Entry` is a public enum in a `0.x` crate and a new variant is
  additive (an exhaustive `match` on it in a consumer breaks, which is why this is named in the decision file as well);
  the log's own `line()`, `cue()` and `at_s()` need arms.
- `line()`: `"0.00s seeded the stream with 0x0000000000000007"` — a person reading the log sees the moment the
  sequence restarted and which seed it restarted from.

### B. The stinger's hold

`src/music.rs`, `play_stingers`:

```rust
// Before: (target, by_db, now, hold_s = 0.0, fade_s = duck_s) — a swell under the stinger.
// After:  (target, by_db, now, hold_s = duck_s, fade_s = DEFAULT_FADE_S)
clock.manual("Music", request.duck_db, now, request.duck_s, DEFAULT_FADE_S);
```

`DEFAULT_FADE_S` is 0.12 s and is the crate's existing release for every stop and every steal; using it here gives
the stinger a step-down, a floor for `duck_s`, and a short click-free release. A field for the release is the decision
file (`stinger-release.md`), not this iteration's change.

`docs/research/siren.md`'s parameter table gains one row (**design default** for the 0.12 s release) and `src/music.rs`'s
`Stinger` docs name all three numbers.

### C. The `MusicLayers::fade_s` fix is *not* in this iteration

Named in the log's next-5 with what it costs: `Pending.fade_s` needs to reach `move_layers`, which means `LiveLayer`
carrying a `fade_s` (or `MIX_TAU_S` becoming `fade_s / 3`, the time constant that settles in `fade_s`). Both are
internal — no public field moves — so the next iteration can take it without a decision file.

## Every field touched on a serde type

| type | field | default | citation |
|---|---|---|---|
| `SirenSettings` | `seed` — **no change**, first time it is read | `0x5EED_51AE_0000_0001` | unchanged; crate rule |
| `MusicScore` / `CueTable` | — | — | untouched |

No new field is added to `CueTable`, `SirenSettings` or `MusicScore` this iteration. The stinger's release is proposed
in `needs-decision/stinger-release.md` and the shipped value borrows `DEFAULT_FADE_S`.

## Tests that prove it

`tests/play.rs`:

1. `a_projects_seed_reaches_the_stream` — two apps, `seed: 7` and `seed: 99`, eight plays each of a three-variant
   `Repeat::Shuffle` cue; the two `Played` streams differ, and each replays itself. **Fails on the unfixed tree** (the
   probe already shows the streams identical).
2. `the_same_seed_replays_the_same_played_stream` — the same app twice, every `Played` field compared (`cue`, `sound`,
   `bus`, `gain_db`, `pitch`); this is case 17's own observation, not the RNG's.
3. `changing_the_seed_restarts_the_stream` — one app, plays under seed 7, then `SirenSettings::seed = 99`, then plays
   again: the stream continues from the new seed (not from where the old one left off), the variant histories are
   reset, and the log has one `Seeded` line naming 99.

`tests/music.rs`:

4. `a_stinger_holds_the_music_back_for_its_whole_length` — at 60 fps, a `Stinger::new(..).duck_db(-9.0).duck_s(0.6)`:
   `SirenMix::ducked("Music")` is −9 dB at the frame it fires, still −9 dB at 0.5 s (inside the hold), and 0 dB by
   `duck_s + DEFAULT_FADE_S + one frame`. **Fails on the unfixed tree** at the 0.5 s assertion, where the duck has
   already climbed to −1.5 dB.
5. `a_stinger_still_plays_immediately` — the existing rule, kept green: the cue's `Played` is on the frame of the
   message, not on the bar.

`tests/model.rs` — no change: `active_duck_db` is the shape and is already pinned.

## Verification

- the whole step-1 baseline and the `cues.yaml` table;
- `cargo run --release --example perfcheck` before and after at 8/64/256 (a change must not cost an untargeted case
  10 %);
- `examples/render` before and after: the cue WAVs must stay byte-identical (this changes no gain a cue plays at) and
  the bus envelope plot must show the stinger's well — added to the render harness as a second bus script, because
  the loop's evidence rule wants a plot of the thing that changed;
- `examples/music` still prints every transition on a bar, unchanged.

## Risk

`seed_choices` compares a `u64` per frame and touches nothing else; it is a branch, not a rebuild. The stinger change is
one argument order. Neither touches a per-voice path, so `perfcheck` should be flat, and the render WAVs should not
move at all — if either does, the change did something it did not intend.
