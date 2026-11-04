# Iteration 121 — plan

## The pick, in one sentence

**A seed is not seeded: `Rng::new(1)` starts a stream whose first draw is zero, so every small seed opens on the first variant (3000 of 3000), neighbouring seeds are correlated for dozens of draws, and seeds related by xor are exactly xor-related streams — against two sentences that promise the opposite** (*"Two projects that differ only in `seed` play different variants and different detunes from the first play"*, `docs/guide/cues.md:211`; *"two seeds are two streams from the first play"*, `README.md:133`). It beats the runner-up (authoring refusals are silent on a console nobody reads the log of: a missing loudness, not a wrong answer) because it is **wrong output on a documented guarantee**, reproduced over 3000 seeds, affecting every game that sets a small seed — which is the natural way to set one, and what a replay, a capture and an A/B all invite.

## Ranked

| # | item | rank | why here |
|---|---|---|---|
| 1 | **Seed the generator: scramble the seed in `Rng::new`** | 1 (wrong output) | reproduced over 3000 seeds; Brent 2007 §4 names it; one root cause, one place; changes every seeded stream |
| 2 | Authoring refusals are printed once per cue and reason (filterable), never `VoiceLimit` | 3 (ergonomics) | a design question with a measurement first (`needs-decision/log-cursor.md`, option 4) |
| 3 | `.on(owner)` of a spatial cue with no `.at` is placed at the owner; the level law follows a moving owner | 2 | works today, flat |
| 4 | The layers of a score start together | 2 | 3 of 100 warm-cache starts, one frame apart |
| 5 | A person listens, and the waiting items | — | |

## What is carved off

* **A better generator.** xorshift64's own statistical weaknesses (linear-complexity artefacts) are not shown to matter for two draws a play, and *"deterministic xorshift64"* stays true. Replacing it (SplitMix64 itself, xoshiro256++, PCG) would be a second stream-changing decision with no defect behind it.
* **`exact-pitch-draw.md`'s option B** (skip the draw an exact pitch range cannot use): a separate stream-changing fix, recommended there *"at the next minor"*. **This change and that one are best shipped in the same minor, so a project's soundscape is re-rolled once** — recorded in `needs-decision/seed-scramble.md`.
* **A stream-version setting** (so a replay recorded on an earlier build could be played on this one) and **independent streams per cue**.
* **Seeding from the clock**: the crate never does (`Rng`'s doc: *"a sound that cannot be reproduced cannot be debugged"*).

## The design

### One scramble in `Rng::new`

```rust
// src/model.rs — engine-free, u64 arithmetic only
pub fn new(seed: u64) -> Self {
    let built_from = if seed == 0 { Self::ZERO_SEED } else { seed };
    // A seed is a name, not a state: the xorshift step is linear over F2, so a seed with few bits set is a state with few bits set (Brent 2007 §4) and seeds related by xor are streams related by xor.
    let mut state = splitmix64(built_from);
    // The mixer maps exactly one seed, `−γ`, to 0, and xorshift never leaves 0.
    if state == 0 {
        state = splitmix64(Self::ZERO_SEED);
    }
    Self { state, built_from }
}

/// The first output of SplitMix64 started at `seed`: a Weyl step (`+ γ`, γ = 0x9E3779B97F4A7C15) and Stafford's variant-13 finalizer.
fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
```

* **`seed()` is unchanged** — the number the stream was built from (`0` still reports `ZERO_SEED`), so `SirenChoices::seed()`, `seed_choices`, the report, the log and the panel agree as before; **`ZERO_SEED`, `default_seed()`, `SirenSettings::seed`'s schema and the RON are unchanged**; `Rng::new`'s signature is unchanged.
* **`next_u32` and `next_f32` are unchanged**: the generator is the same xorshift64 (13, 7, 17), started from a mixed state.
* **Every seeded stream differs from before, including the default seed's.** Same-seed determinism, restart-on-seed-change and the `0` alias are untouched. `needs-decision/seed-scramble.md` records it as a release note (precedent and its logic: `exact-pitch-draw.md`).
* **New `Rng` doc**, `SirenSettings::seed` doc, `docs/guide/cues.md` (*The seed* and the `0` paragraph, whose stated reason — *"xorshift's state must never be zero"* — is now the reason for the alias being kept, not for it being needed), README, `CHANGELOG.md` (**Changed** *and* **Fixed**), `AGENTS.md` (a trap), `cues.yaml` case 42, the skill.

## The tests

In `tests/model.rs` (engine-free, beside the existing seed tests; deterministic: a fixed algorithm either clears a bound or does not):

1. **`a_seed_opens_a_stream_the_way_a_random_seed_would`** — the first pick of k ∈ {2, 3, 4, 8} variants over seeds 1 … 3000 clears the chi-square 0.1 % bound (10.83 / 13.82 / 16.27 / 24.32); the first draw over seeds 1 … 4096 in 16 bins clears 37.70. *(Red on the unchanged tree: χ² = 3000 × (k − 1) and 61 440.)*
2. **`neighbouring_seeds_are_uncorrelated_draw_for_draw`** — |Pearson r| < 4/√(N − 1) between draw *n* of seed s and s + 1, n ∈ {1, 2, 3, 4, 6, 8, 16, 32, 64}. *(Red: 0.999 … 0.40.)*
3. **`one_flipped_seed_bit_flips_half_the_first_draw`** — mean Hamming distance 16.0 ± 0.1 over 64 positions × 256 seeds, every position in [15, 17]. *(Red: 3.78; bits 0 and 1 at 0.)*
4. **`related_seeds_are_not_related_streams`** — for no triple (a, b, a ⊕ b), a < b ≤ 63, does draw *n* of the third equal the xor of the other two for all of the first 16 draws. *(Red: all 1953.)*
5. **`splitmix64_gives_the_reference_outputs`** (unit test in `src/model.rs`, since the function is private): from state 0 the outputs `e220a8397b1dcdaf`, `6e789e6aa1b965f4`, `06c45d188009454f` (and `splitmix64(1) = 910a2dec89025cc1`) — independent of this crate's code.
6. **`the_stream_is_the_one_the_docs_describe`** — known answers for seeds 1, 2, 3, 42, the default and `u64::MAX` (first four `u32` draws, from the independent Python implementation): **a change to the stream is a release note, so this is the test that makes it a deliberate act** (`exact-pitch-draw.md`).
7. **`the_one_seed_that_would_zero_the_state_still_draws`** — `0x61C8864680B583EB` gives 64 draws, nearly all distinct, none zero, and the default stream.

In `tests/play.rs`, **through the real play path** (the claim is about what a *game* hears): **`a_cue_opens_on_any_variant_across_small_seeds`** — seeds 1 … 300, one play of a three-variant cue each: the first variants within the 13.82 bound, and the first detune over four equal bins of its range within 16.27. *(Red: 300 of 300 on the first variant.)* The five existing seed tests pass **unedited**.

**Mutations** (each switch applied, the seed tests run, put back): M1 no scramble; M2 the *reported* seed scrambled too; M3 no zero-state guard; M4 one bit of a finalizer constant wrong; M5 a weaker mixer (one multiply, no second xorshift); M6 Fibonacci hashing instead (`seed × γ`: not a mixer).

## The gate

`examples/render`: **the 21 WAVs byte-identical** (it draws a cue's speed at `pitch.at(0.0)`), and only `render.txt`'s `variants …` lines change; `mix.wav` loudness and its spectrogram **read** (the picture does not change, which is the point); `perfcheck` ×3 and `Rng::new` timed directly; the probe re-run on the final tree (`evidence/after/`); the quick start's first step on the default seed (**P9**: `step_01.ogg`); a picture (`docs/media/seeds.png`, the tally before and after).

## Modules touched

`src/model.rs` (`Rng::new`, `splitmix64`, docs), `tests/model.rs`, `tests/play.rs`; docs: `docs/guide/cues.md`, `README.md`, `src/mix.rs` (the `seed` field's doc), `CHANGELOG.md`, `AGENTS.md`, `docs/loop/cues.yaml`, `docs/loop/needs-decision/seed-scramble.md`, `docs/media/`, the skill. **No change to a system, a schedule, a message, a schema or a signature.**
