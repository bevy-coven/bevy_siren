# review.md — iteration 23, three passes

## Pass 1 — function

**The claim.** `SirenChoices::seed()` is documented as *"the seed this stream was built from"* and reported the seed it was
**asked** for. Those differ for `0`, because `Rng::new(0)` maps a zero state to `ZERO_SEED` — which is also
`SirenSettings::default_seed()`.

| row | source | before | after |
|---|---|---|---|
| `seed: 0` | S1 (`Rng`'s doc: a `0` seed maps to a constant), S2 (`seed()`'s own sentence) | `seed()` → **`0`** for a stream built from `0x5EED…` | `seed()` → **`0x5EED51AE00000001`** |
| the default seed | S2 | `0x5EED…` | `0x5EED…` — unchanged |
| `seed: 1` (guard) | — | `1` | `1` — unchanged |
| the two streams | S3 (`cues.yaml` case 17) | **identical**, reported under two different numbers | identical, reported under **one** |

**Against the sources.** S1 is why the mapping exists and why `0` cannot be its own stream; S2 is the sentence that was
false. The fix is to **report the mapping**, not to hide it: a project that wants a stream unlike the default writes any
seed but `0`, and now the crate says which number it is hearing.

**A defect in my own first attempt, caught by a test I wrote for it:** `Rng::seed()` initially read `self.0`, the live
xorshift state — which changes on every draw, so the seed would have reported a different number after each play.
`a_streams_seed_does_not_move_as_it_draws` exists because I made that mistake, and it fails against the version that
reads the state.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/seed{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**Audio unchanged, and it must be.** No sound changes: the mapping already existed, and this iteration only stops the
report from hiding it. 21/21 WAVs byte-identical; `mix.wav` −4.3 LUFS / +0.2 dBFS, and the seed probe's own streams are
identical before and after for all three seeds.

**The measured table**, `evidence/seed{,-before}.txt`:

```
before                                          after
seed 0      -> seed() = 0        (stream k2 k0 …)   -> 6840213216811810817
default     -> seed() = 6840213216811810817        -> same
seed 1      -> seed() = 1                          -> 1
```

Two identical streams reported under **two different numbers** before; one number after.

**Also measured, and recorded as no-item:** the `Rng` stream is uniform (4-bin histogram over 400 000 draws:
99754 / 99824 / 100469 / 99953) and its 8-draw window did not repeat in 20 000 000 draws. `next_f32` never reaches 1.0
(max 0.999999583 over 2 M draws from `u64::MAX`). So `pick`'s index clamp is defensive, and the doc's *"xorshift64\*"* is
a **naming** fault — the code is Marsaglia's plain xorshift64 — corrected in the doc string only.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The tests reach the seed by three doors — `SirenChoices::new` directly, `Rng::new` directly, and
through `SirenPlugin` where `seed_choices` runs — and one test pins the **stream identity** (`seed: 0` and the default draw
the same 32 numbers) so a future change that split them would be visible as a behaviour change rather than a quiet one.

**Can a developer understand it without the code?** `Rng`'s doc now names `ZERO_SEED`, says it is also the crate's default
seed, and says that `seed()` answers the **effective** seed. `SirenChoices::seed()`'s sentence is true by construction —
it records `rng.seed()`.

**What a fresh reader would still ask:** *why not make `0` its own stream?* Because the state must never be zero, and
`ZERO_SEED` is the fix for that; reporting it is the honest answer. Written in the guide.

**Verdict: GREAT.**
