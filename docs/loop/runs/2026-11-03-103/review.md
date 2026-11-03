# Iteration 103 — review

## Pass 1 — function

**What the item is.** Not an open issue: a defect in shipped code. `#20`'s criterion 2 required the override be *"visible
in the query result"*, and iteration 100 closed it by storing `Index::overridden` and documenting that every search would
carry it. **No search read it**, and both refused on any stamp mismatch.

| claim | before | after |
|---|---|---|
| an overridden index can be queried | **`Some(StampMismatch)`** — measured | ranks its vectors |
| the override is visible where a result is built | nowhere | `Index::accepts` is the single place that decides |
| consent is narrow | not applicable — nothing worked | pinned to the exact model it was granted for |
| a plain index still refuses | yes | **yes, unchanged** |

**The design decision worth arguing: why the override is pinned.** An index overridden for `version b` is refused under
`version c`, asserted by `strict.accepts(&third) == false` and by `search_ranked` answering `StampMismatch`. A recorded
override that honoured *any* model would be a standing permission to rank noise — the exact failure `docs/adr/0004`
exists to prevent, reached by a different road. The user consented to one mismatch, and the code now honours exactly that.

**Why it lives on `Index`.** Two entry points check it. A rule written twice is a rule that eventually differs, and this
very bug is what that costs — the stamp rule existed in two copies and the override was added to neither.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/ui-cancel.png` and `spectrograms/voices8-wave.png`.

- `ui-cancel.png`: a 0.025 s attack across the full band, decaying into a low-frequency skirt that settles by 0.06 s, then
  a flat floor to 0.33 s. **No vertical line**, no shelf, no hole — and a UI sound is exactly where a click would be
  audible, so this is the right clip to check.
- `voices8-wave.png`: eight discrete bursts with clean silence between them, no clipping plateau.
- **All 21 rendered wavs byte-identical** to iteration 102 (`cmp`), and master loudness unchanged at **−21.5 LUFS,
  −7.5 dBFS true peak**.

A predicate cannot change audio, and the byte comparison is the stronger statement: not *similar*, the same bytes.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Read as the user the doc addresses: *"a user who knows what they are doing"*.

- They get a refusal naming both versions, follow its advice (`load_allowing(.., true)`), and now get **a working index**
  instead of a second refusal that contradicts the first. The two refusals would have been the worst outcome — a user told
  to do something, doing it, and being told again.
- Read as a developer: `accepts` is four lines and its doc states both clauses and the reason for the second. A reader does
  not need the bug report to understand why the pinning exists.
- Read as a maintainer: the rule is in one place, so a third search entry point added later calls `accepts` or does not
  check the stamp at all — a visible omission rather than a silent divergence.

**One criticism, recorded rather than fixed:** `search.rs` still maps a refusal to `SearchProblem::StampMismatch` without
saying *which* stamps differed. `IndexProblem::ModelMismatch` carries both; `SearchProblem` carries neither. A caller
seeing `StampMismatch` cannot tell the user what to re-index to. Carrying the pair would mean a new field or variant on a
public enum, which the crate's rules send to `needs-decision/` — so it is named as a follow-up rather than smuggled in.

**Verdict: GREAT.**
