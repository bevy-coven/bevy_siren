# Should the diagnostic enums be `#[non_exhaustive]`?

## The question

`CueProblem`, `Refusal`, `WeightProblem`, `IndexProblem` and `LoadProblem` are **public** enums with no
`#[non_exhaustive]`. Each has gained a variant after shipping, and each time an exhaustive `match` somewhere stopped
compiling.

Iteration 105 is the fourth time, and it has a measurement attached:

```
error[E0004]: non-exhaustive patterns: `&CueProblem::InertAvoidRepeat { .. }` not covered
  --> tests/table.rs:66
```

That was **in this repository** — `tests/table.rs` matched every `CueProblem` to assert that each names its cue. The test's
subject is *"each problem is a sentence naming its cue"*; enumerating the variants was incidental, and adding one broke it
for no reason connected to what it checks.

A consumer outside this repository pays the same cost with less information: `refusal-names-its-cap.md` records that
matching `Refusal` exhaustively is how a host reacts to a refusal by kind, and every new variant is a compile error in
that host.

## What each option costs

1. **Add `#[non_exhaustive]` to all five.** Cost: **one breaking change, once**, for any consumer that matches
   exhaustively — the same break it would take on the next variant anyway, but paid once instead of per variant. A
   `match` must then carry a `_ =>` arm, which is honest: these are **diagnostics**, and a host that does not recognise a
   new one should print it, not fail to compile. Benefit: every future variant is additive, which is what a diagnostic
   enum should be. Limitation, stated precisely: `#[non_exhaustive]` is ignored *inside* the defining crate, so the
   crate's own `match`es stay exhaustive and a new variant is still a compile error there — the right pressure, since the
   crate must decide what it does with its own new problem. It **is** honoured in every other crate, **including this
   repository's `tests/`** (the `all` binary is a separate crate from the library): `tests/table.rs`'s `kinds!` macro
   would need a `_ =>` arm, and its guard that every variant has a fixture would then have to assert that nothing falls
   into that arm.
   One line; the coverage half of the guard survives.
2. **Leave them closed, and pay per variant.** Cost: a compile error in every exhaustive consumer, per variant, forever
   — measured above as one in this repository, and recorded as the host cost in `refusal-names-its-cap.md`. Benefit: a
   consumer is *forced* to consider a new variant, which is a real property for `Refusal` (a game may have to react to
   `VoiceLimit` differently from `UnknownCue`) and no property at all for `CueProblem` (a table author reads a sentence).
3. **Split by audience: `Refusal` stays closed, the rest become `#[non_exhaustive]`.** Cost: the same one break, applied
   unevenly, and a rule a contributor has to remember: *"is this enum a message a game branches on, or a sentence a
   person reads?"* Benefit: it matches how each is actually consumed — `Refusal` is control flow, `CueProblem` and
   `WeightProblem` are status lines, `IndexProblem` and `LoadProblem` are somewhere between.
4. **`#[non_exhaustive]` plus a `severity()` or `kind()` method** on each, so a consumer has a stable way to branch
   without matching every variant. Cost: more surface to design and document. Benefit: it makes option 1 comfortable for
   `Refusal` too, because the branchable property becomes a method rather than the variant set.

## The recommendation

**Option 3, applied at the next minor**: `Refusal` stays closed because it is the one enum a game genuinely branches on,
and `CueProblem`, `WeightProblem`, `IndexProblem` and `LoadProblem` become `#[non_exhaustive]`.

The distinguishing question is worth writing down as a rule: **an enum a consumer branches on is closed, because a new
variant is a new decision for that consumer; an enum a consumer prints is open, because a new variant is a new sentence
and the consumer's job does not change.**

Iteration 105 shipped `CueProblem::InertAvoidRepeat` the closed way — the variant exists, its sentence exists, and
`tests/table.rs` compiled again once an **arm was added** to a closure that already listed every variant. **An earlier
version of this paragraph called that "fixing the test to match the documented property rather than the accident". It was
not:** the closure still enumerated every variant, and it was vacuous besides (`problems.contains(&problem)` already
implied it). Iteration 108 found that while reading the diff, and found the consequence: the compiler's demand for an arm
*was satisfied* and the variant still never got a fixture, so the suite stayed green with seven variants and six rows.
`tests/table.rs` now lists each variant **once**, in a `kinds!` macro that generates the exhaustive `match` and the count
together — so the enumeration is deliberate (its job is *"every kind has a fixture"*), which is a different property from
the one the closure pretended to check, and turns the 105 gap from a silent omission into a failing test.

## What is blocked

Nothing. Iterations may add variants the closed way, paying the documented cost, until this is decided.
