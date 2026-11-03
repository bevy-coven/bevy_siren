# Where per-asset provenance lives, and how a result reports it

- **Issue:** [#14](https://github.com/bevy-coven/bevy_siren/issues/14)
- **Status:** recommends, does not decide
- **Depends on:** ADR 0004 (the store), ADR 0005 (the licence audit)

## What is already built

`semantic::provenance` exists and is tested: a `Provenance` type with `licence` / `source` / `author` as `Option`s, a
`Sidecar` reader for the CSV and JSON shapes a pack ships, and `permits_commercial_use() -> Option<bool>` whose `None`
is *"a person must decide"* rather than *"yes"*.

**The type is additive. These two storage questions are not**, which is why they are here:

1. **The index has no place for it.** `Index::records` is `BTreeMap<u64, Vec<f32>>` — hash to vector — and its header
   is the literal `"bevy_siren index 1"`.
2. **`rank::Match` is `Copy`.** It is `{ hash: u64, score: f32 }`; attaching a `Provenance` with two `String`s makes it
   neither `Copy` nor cheap, and every caller that passes it by value changes.

## Question 1 — where provenance is stored

### A. A `provenance` section in the index, beside the vectors

```
bevy_siren index 1
count <n>
checksum <hex>
<hash> <dim floats...>
...
provenance <m>
<16 hex hash> <licence> <source-kind> <source> <author>
```

- **Backward compatible**: a file that ends after the vectors loads exactly as it does today, so `MAGIC` does not move.
- **Absence is absence**: an asset with no record is simply not in the section, which is the same `None` the type draws.
  A sentinel in a numeric column is how `−70 LUFS` became a lie in `#21`; this avoids repeating it.
- **Cost**: a second count and a checksum to keep honest, and a second pass on load.

### B. A fourth field on each record line

- **Cost**: breaks every written index — `MAGIC` must become `index 2` and a user loses their index once.
- **Cost**: the licence is a free-form string, so the line becomes `hash v... licence`, and a licence containing a
  space needs quoting in a format that has no quoting.

### C. A sibling file, `<root>/siren-provenance.csv`

- **Cost**: two files a user can separate, and a checksum that covers one.
- **Benefit**: the index format does not move at all, and a person can edit the CSV.
- **Risk**: "index present, provenance missing" becomes a state every caller must handle.

**Recommendation: A.** It is the same shape ADR 0004 already chose for loudness — an optional trailing section, absence
encoded as absence — so the two features share one rule instead of inventing a second. B is disqualified by the free-form
string in a whitespace-delimited format; C by having no single file to checksum.

## Question 2 — how a search result reports it

### A. `Match` gains `provenance: Provenance`; it stops being `Copy`

```rust
pub struct Match {
    pub hash: u64,
    pub score: f32,
    pub provenance: Provenance,   // two Option<String>
}
```

- **Cost**: `Match` is no longer `Copy`, so callers iterating by value change to by reference. Within this crate that is
  `rank::top_k`'s return type only; it is a **semver-minor break** for a consumer that relied on `Copy`.
- **Benefit**: the result carries what the issue asks for — *"a search result can report an asset's licence and source"*
  — with no second lookup and no way to forget it.

### B. `Match` gains `provenance: Option<Box<Provenance>>` to stay small

- **Cost**: an allocation per result, and a `Box` a caller must dereference for a struct of two pointers.
- **Benefit**: `Match` stays small; it still cannot be `Copy`.

### C. `Match` keeps `{ hash, score }` and a caller looks provenance up separately

- **Cost**: every consumer must remember to, and the one that forgets presents an unlicensed asset as if it had terms.
  That is the failure the issue's fourth criterion names.
- **Benefit**: no public-type change at all.

**Recommendation: A.** C is the option to refuse explicitly: it makes the obligation opt-in at the one place the issue
says it must be unavoidable. Between A and B, A costs a `Copy` that only this crate's own `top_k` uses, and B buys a
box for a struct that is already two pointers and a discriminant.

## What each choice costs a consumer

| | 1A (index) | 1B (index) | 1C (index) | 2A (result) | 2B (result) | 2C (result) |
|---|---|---|---|---|---|---|
| existing written index keeps working | **yes** | **no** | yes | — | — | — |
| an asset with no record | **absent** | sentinel | absent | — | — | — |
| `Match` stays `Copy` | — | — | — | **no** | **no** | yes |
| a result cannot forget its licence | — | — | — | **yes** | **yes** | **no** |
| allocation per result | — | — | — | none | **one box** | none |

## Why this is not decided here

Both are **schema changes** — one to the definitions file, one to a public type — and this crate's rule is that those go
to a decision file. The measurement they rest on is already built and tested, and does not change with the answer.
