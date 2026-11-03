# Does `WeightProblem` carry the field it is about, in one variant or many?

## The question

`WeightProblem` is a **public** enum with a per-variant payload. Three checks used to push `BadDropFraction` for problems
that were not about `drop_fraction`, so a project that set `clash_penalty: 1.5` was told:

```
`drop_fraction` is 1.5: it must be at least 0 and below 1, because it is the share of results to drop
```

— sent to edit a field it had not touched. Iteration 104 replaced those three pushes with `BadMultiplier { field, value }`
and `BadSimilarity { value }`, which **adds two variants to a public enum**.

## What already shipped, and why it was safe

`WeightProblem` is **not** `#[non_exhaustive]`. Adding a variant is therefore a **breaking change** for any consumer that
matches it exhaustively. The crate decided it rather than filed it — a decision this file now puts in front of the owner,
with the reasoning that justified it:

- `WeightProblem` is produced by `Weights::problems()` and consumed by a **display loop** — the doc says *"a file that says
  it impossible is a message and not a silence"*, and a caller's job is to print it;
- a consumer matching every variant in order to *branch* would already be reading a diagnostic enum as control flow, which
  the crate's own `Refusal` doc argues against;
- the crate is **0.1.0**, where a minor may break;
- and the alternative — keeping one variant and rewriting its sentence — cannot work: the sentence is `Display`'s, and it
  has one `value` to print, so it must name a field, and one field name cannot be three different fields.

## The options

1. **Two new variants, as shipped.** `BadMultiplier { field: &'static str, value: f32 }` and `BadSimilarity { value: f32 }`.
   Cost: an exhaustive `match` in a consumer breaks. Benefit: every message names the field it is about, and the string is
   `&'static str` from the crate rather than a `String`, so a message cannot be built from caller data by accident.
2. **One variant with a field and a range.** `BadSetting { field: &'static str, value: f32, min: f32, max: f32 }`, replacing
   `BadDropFraction`, `BadMultiplier` and `BadSimilarity`. Cost: a break of the same size, plus `BadDropFraction`'s existing
   payload disappears, so a consumer reading `value` must change too. Benefit: three variants collapse to one, and the
   **range travels with the value**, so a consumer can validate before calling rather than reading a sentence.
3. **No new variant: keep `BadDropFraction` and rewrite its sentence.** Cost: none. Benefit: none that matters — the
   sentence has one `value`, so making it true for three fields means printing *"a value of 1.5 is out of range for
   `drop_fraction`, `clash_penalty` or `parallel_penalty`"*, which is a message that names three fields and therefore tells
   a person nothing. This is the state the crate was in, and it is the reason the issue exists.
4. **Mark the enum `#[non_exhaustive]` and add variants freely from now on.** Cost: breaking **once**, for every consumer,
   including those that do not match exhaustively today. Benefit: never again. Limitation: it does not fix the three
   messages already wrong; it only stops the next iteration paying for it.

## The recommendation

**Option 1 short term, option 4 at the next minor.** Option 1 is what makes the shipped messages true, and the payload is
`&'static str`, so nothing a caller writes can reach a message. Option 4 is the structural fix: this is the **fourth**
diagnostic enum in this crate to gain a variant after shipping (`Refusal`, `CueProblem`, `IndexProblem`, `LoadProblem`),
and each time the same cost was paid. Making `WeightProblem` and its siblings `#[non_exhaustive]` once is cheaper than
paying per variant forever.

Option 2 is genuinely attractive and would be my choice if the enum were being designed today; it is not worth a second
break to consolidate variants that already work.

## What is blocked

Nothing. Iteration 104 shipped option 1 with tests, and every message now names its own field:

```
`clash_penalty` is 1.5: it multiplies a score, so it must be between 0 and 1.
`min_similarity` is 2: a cosine only reaches from -1 to 1, so this floor would drop every candidate.
```

If the owner prefers option 2 or 3, the change is confined to `weights.rs` and its tests.
