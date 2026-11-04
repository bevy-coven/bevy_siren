# `Rng::new`: a seed is mixed before the generator sees it — and every seeded stream changes once

*Iteration 121, 2026-11-04. A public function is added and **every seeded stream changes**; built under the standing "recommend, record, proceed" rule (`assumed-decision`). The precedent for why a stream change is a release note and not a patch is [`exact-pitch-draw.md`](exact-pitch-draw.md).*

## What changed in the public surface

- **Added**: `model::splitmix64(seed: u64) -> u64` — the first output of SplitMix64 from `seed`, public so a test can hold it to the published outputs.
- **Unchanged**: `Rng::new`'s signature, `Rng::seed()`, `Rng::ZERO_SEED`, `Rng::next_u32`/`next_f32`, the generator (xorshift64 with the shifts 13, 7, 17, the top 32 bits), `SirenSettings::seed`'s type, default and RON, `SirenChoices::seed()`, and every message and schema.
- **Behaviour**: the stream a given seed plays is **different, for every seed, including `0` and the default**. The same seed on the same build still plays the same sounds field for field, and a seed change still restarts the stream.
- **What a consumer sees**: a project that never set a seed hears a different first variant and different detunes on the next build (the quick start, on the default seed: `step_01.ogg` first, where it was `step_02.ogg`); a capture, a replay or a golden-audio comparison recorded on an earlier build plays other sounds once; a `seed:` in a RON file keeps its meaning (the number) and not its sounds.

## Why

`Rng::new(seed)` was xorshift64 whose state **was** the seed. Measured on the unchanged tree over seeds 1 … 3000 (`runs/2026-11-04-121/evidence/before/seed-probe-on-the-unchanged-tree.txt`): the first pick of a cue was its **first variant for 3000 of 3000** (at k = 2, 3, 4 and 8); the first draws of neighbouring seeds correlated at **r = 0.9995** and draw *n*
stayed correlated to n = 64 (−0.78 at n = 6); one flipped seed bit changed **3.8 of 32** bits of the first draw; and the streams of the seeds `a`, `b` and `a ⊕ b` were xor-related for all 1000 draws measured, in **1953 of 1953** triples. Brent (2007, §4) names the mechanism — a sparse linear map keeps a low-weight vector low-weight, and seeds related by xor are streams related by xor, *"clearly
undesirable"* — and the cure: *a step that is not linear over F₂*, such as addition of a Weyl increment. The crate's own words said the opposite (*"Two projects that differ only in `seed` play different variants and different detunes from the first play"*, `docs/guide/cues.md`; *"two seeds are two streams from the first play"*, README).

## Options

| # | option | what it fixes | cost to a consumer | verdict |
|---|---|---|---|---|
| 1 | **Run the seed through SplitMix64's mixer in `Rng::new`; keep xorshift64** (this change) | all four measured defects, at their root, in one place; `0.7 ns` once per seed change | every seeded stream changes once, the default's included | **chosen** |
| 2 | Replace the generator with SplitMix64 itself | the same four; also xorshift's own linear artefacts | the same re-roll, and *"xorshift64"* leaves the docs | no: a second stream change with no measured defect behind it; carved off |
| 3 | Mix in `SirenChoices::new` only; leave `model::Rng::new` alone | the crate's own streams | `Rng::new(small)` stays a trap for every other caller (the render harness, a test, a game using the public `Rng`); the default stream changes anyway | no: the defect is in the generator's constructor |
| 4 | Discard the first *N* outputs (a warm-up) | the first draws only | none (the streams still change) | no: it does not cure the xor relation between related seeds' streams, which holds for the **whole** stream (Brent §4) |
| 5 | Leave it and document *"use a seed with its bits spread out"* | nothing | every author carries it; the guide's own sentences stay false for small seeds | no |
| 6 | Mix only a seed that *looks* small | the measured case | the stream becomes a function of how a seed looks; a discontinuity at the threshold | no |

## Release timing

[`exact-pitch-draw.md`](exact-pitch-draw.md) recommends skipping a pitch draw an exact range cannot use *"at the next minor"*, for the same reason: it moves every seeded stream. **Ship the two in the same minor, so a project's soundscape is re-rolled once and not twice.** This change is in `main`; **the release is a separate act** and
the `CHANGELOG.md` entry is its release note. The crate promises *same seed, same sounds* **on a build**, and now says so (`docs/guide/cues.md`); `tests/model.rs::the_stream_is_the_one_the_docs_describe` pins the first four draws of seven seeds from an independent implementation, so the next change to the stream is a decision with a changelog line and not a refactor.

## Edge cases and what is decided

- **The one seed whose mixed state is zero.** The mixer is a bijection with 0 ↦ 0, so exactly one seed, −γ mod 2⁶⁴ = `0x61C8864680B583EB`, would give the one state xorshift never leaves (without the guard its stream is all zeros: mutation M3). It plays the **default** stream. (Odds that a game meets it by chance: one in 2⁶⁴.)
- **`0` stays an alias of the default seed**, `seed()` still reports `ZERO_SEED`: documented and tested, and moving it too would be a second change. With the seed mixed, `0` would be a perfectly good seed of its own.
- **`seed()` reports the seed written**, not the mixed state: `seed_choices` compares it with `SirenSettings::seed` every frame, and a reported seed that differs **restarts the stream every frame** (mutation M2 reddened twelve tests: three about the report, nine about the picks).
- **No stream-version setting.** A project that needs to replay a capture across builds would need one; none has asked, and it would be a schema change the editor reads. Recorded, not built.
- **Cost**: `Rng::new` 0.7 ns, a draw 1.5 ns (unchanged); `perfcheck` does not draw.

## Recommendation

Option 1, as built, released in the same minor as `exact-pitch-draw.md`'s option B. Revisit a stream-version setting only if a consumer needs cross-version replays.
