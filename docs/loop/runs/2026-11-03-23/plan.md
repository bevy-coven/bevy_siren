# plan.md — iteration 23: report the seed the stream came from

## The pick, and why it beats the runner-up

**Pick:** `SirenChoices::seed()` is documented as *"the seed this stream was built from"* and reports the seed it was
**asked** for. For `seed: 0` those differ, because `Rng::new(0)` maps a zero state to `0x5EED_51AE_0000_0001` — which is
also `SirenSettings::default_seed()`. So `seed: 0` and the default are one stream under two names, and the method that
exists to say which stream you have says the wrong number.

Beats the runner-up (an empty `sound` key is accepted and played with no `CueProblem`): that one needs a **seventh
variant of an exhaustive public enum**, and iteration 22's decision file is already open on the same cost. This one
needs no public type changed, and it is the *reproducibility* promise — the crate's own stated reason for having a
seeded stream at all (*"a sound that cannot be reproduced cannot be debugged"*).

## The design

1. **`Rng::seed()`** — the **effective** seed: `0` mapped to the constant, else the seed. One definition of "which seed is
   this stream", in the type that owns the mapping.
2. **`SirenChoices::new(seed)` records `rng.seed()`** in `seeded`, so `seed()` returns the number the stream came from and
   `seed_choices`' comparison is against that same number. A project that writes `seed: 0` is told
   `0x5EED_51AE_0000_0001`; one that writes `7` is told `7`.
3. **Docs**: `Rng`'s doc says what `seed()` answers and why `0` is not a stream of its own; `SirenChoices::seed()`'s
   sentence becomes true.

**No public type changes.** `Rng` and `SirenChoices` both gain a method (or a corrected one); no field shape changes, no
enum variant, no serde schema.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `SirenSettings::seed` and `default_seed()` keep their values — the
mapping is the existing, documented behaviour, and this iteration only stops the **report** from hiding it.

## Tests

- `a_zero_seed_reports_the_seed_it_used` — `SirenChoices::new(0).seed() == SirenSettings::default_seed()`. **Fails on the
  base commit** (reports `0`).
- `a_nonzero_seed_reports_itself` — the guard: `new(7).seed() == 7`, so the fix is a mapping report and not a blanket.
- `a_zero_seed_and_the_default_seed_are_one_stream` — records the **behaviour** this report describes: the two produce
  identical `Played` streams. Pinning it stops a future change from quietly splitting them (which would be a behaviour
  change needing its own decision) and stops the report from drifting away from the sound again.
- `a_zero_seed_through_the_plugin_reports_the_default` — the same through `SirenPlugin`, where `seed_choices` runs.

## The carve

- **Not** the empty `sound` key: `CueProblem` variant, blocked (`needs-decision/empty-sound-key.md`, written this
  iteration).
- **Not** `Rng`'s doc calling it `xorshift64*`: a naming fault with no behavioural effect (measured uniform, period
  > 20 M); corrected in the doc string only, because the code is Marsaglia's xorshift64.
