# Iteration 104 — research: a filter that silently matches nothing

## The item

Not an open issue: **two defects in `semantic::tools`, found by reading my own tool code against what it promises.**
Both are `#43`'s — *"a category filter that matches nothing returns an empty list with a reason"* — failing in a way the
tests did not catch.

## The two defects, measured

**1. The similarity floor is hardcoded.**

```rust
if similarity < Floor::default().min_score {   // tools.rs:338
```

`Toolset` holds `music`, `effects` and `variants` — **no floor**. `Floor::default().min_score` is `0.0`, so the branch can
never fire: `Tally::below_floor` is always zero, `describe()` never says *"N below the similarity floor"*, and a caller
who wants a stricter floor **cannot set one**. The tally field exists, is documented, is rendered, and is unreachable.

**2. The category comparison is case-broken.**

```rust
fn source_of(source) -> String { … url.to_lowercase() / name.to_lowercase() … }
…
if !source_of(source).contains(want) { tally.wrong_category += 1; continue; }
```

The haystack is lowercased; the **needle is not**. A caller passing `Some("Footsteps")` against a pack recorded as
`Footsteps` — the exact spelling a person reads in their own provenance file — gets an empty list, and the tally blames
*"N outside the category"* rather than saying the filter never could have matched. My iteration-97 test passed only because
it happened to use the lowercase form.

## Sources

| Source | What it reports |
|---|---|
| Sun, Min, Chang & Bisk, *Tools Fail: Detecting Silent Errors in Faulty Tools* — **DOI 10.48550/arXiv.2406.19228** | names this exact failure class and its two halves: **failure detection** — *"recognizing failures and assessing their severity"* — and **fault assignment** — *"identifying which tool caused the error"*. A tool that returns a plausible empty list has failed silently: the caller cannot tell *"nothing matches"* from *"your argument was wrong"* |
| `#43`'s own text | *"a category filter that matches nothing returns an empty list **with a reason**"* — a reason that names the right cause |
| `src/semantic/search.rs` | the crate's existing vocabulary for this: *"no result scored high enough; N were below the floor"* — a sentence that says **which** rule emptied the list |
| `Floor::min_score`'s doc | *"**Design default: 0.0**, which keeps everything"* — a default, not a policy to hardcode |

## The technique

**Give the filter its number, and make the comparison total.**

- The floor becomes a field on the most natural existing type. `Weights` already holds *every* authored threshold the
  tools use — the three tolerances and the drop fraction — so `Weights::min_similarity` belongs there rather than a
  fourth parallel struct. It defaults to `0.0`, matching `Floor::default()`, so **no existing behaviour changes**.
- The category comparison lowercases **both sides**. A category is a string a person typed into a provenance sidecar;
  matching it case-sensitively is a defect the caller cannot see.

**And the fault must be assignable.** If a category filter removes everything, the answer must say **the category** emptied
the list, not that the library had nothing — which is the second half of the paper's framework.

## Parameters

| field | default | source |
|---|---|---|
| `Weights::min_similarity` | `0.0` | `Floor::min_score`'s existing **design default**, moved to a named field. `0.0` keeps everything, which the crate measured as correct: the semantic bands overlap (0.126–0.429 correct against −0.104–0.150 wrong), so a positive default would reject correct matches |

## What good sounds like

A caller asking for `"Footsteps"` gets the footsteps. A caller asking for `"explosions"` against a footsteps-only library
gets an empty list whose sentence says **"N outside the category"** and does not also claim the acoustic rules removed
anything. A caller who raises `min_similarity` sees `below_floor` count what it dropped.
