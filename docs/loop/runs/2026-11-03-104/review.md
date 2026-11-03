# Iteration 104 — review

## Pass 1 — function

**Three defects, in code this loop wrote and closed issues against.** Each is a case where the documented behaviour and
the actual behaviour had diverged and no test compared them.

| defect | the promise | what the code did |
|---|---|---|
| the similarity floor | `Tally::below_floor` documented as *"candidates below the similarity floor"* and rendered by `describe()` | compared against `Floor::default().min_score` = `0.0`, so it could **never** fire; `Toolset` had no floor for a caller to set |
| the category filter | `#43`: *"an authored taxonomy … a category is whatever the caller's file names say"* | lowercased the haystack, compared a **raw** needle. `Some("Footsteps")` matched nothing |
| the multiplier refusal | `WeightProblem`'s doc: *"named so a file that says it impossible is a message and not a silence"* | pushed `BadDropFraction`, whose sentence says *"`drop_fraction` is 1.5"* — **naming a field the author never touched** |

**Against the source.** Sun et al. (`10.48550/arXiv.2406.19228`) name both halves of this failure class: **failure
detection** — *"recognizing failures"* — and **fault assignment** — *"identifying which tool caused the error"*. All three
defects fail the second: the tally said the acoustic rules excluded nothing while the category removed everything, and the
message named `drop_fraction` when the fault was a multiplier.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/footstep.png` and the loudness figures.

- `footstep.png`: a 0.025 s attack across the full band, decaying into a low-frequency body that settles by 0.03 s, then a
  flat floor to 0.33 s. **No vertical line**, no shelf, no hole. A footstep is the clip where a click would be most
  audible, so it is the right one to check.
- **All 21 rendered wavs byte-identical** to iteration 103, master **−21.5 LUFS, −7.5 dBFS true peak**.
- The zoom here is really the **sentences**, since that is what changed. Checked by assertion rather than by eye:
  `a_similarity_floor_beyond_a_cosine_is_refused` asserts the message **contains `min_similarity` and does not contain
  `drop_fraction`**; `a_bad_compatibility_multiplier_names_the_multiplier` asserts the same for both multiplier fields.

**Two of my own test assumptions were wrong and the tests caught them**:

1. I asserted `wrong_category == 0` after a successful match and got **2** — the other two assets have **no provenance at
   all**, so they cannot match any category. The assertion now says so, and it is the honest answer: an asset whose source
   was never recorded is not in a category.
2. I set the "impossible" floor to `0.999_9` and the list was **not** empty — asset 4's cosine is **0.99995**, which is
   how I learned the fixture's own number. The floor is now exactly `1.0`.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Read as a caller with an effect library:

- `Some("Footsteps")`, `Some("footsteps")`, `Some("FOOTSTEPS")` and even `Some("ootste")` all match the one pack — asserted
  as four cases, because the bug was a case bug and one case is what let it through.
- A raised `min_similarity` now drops candidates **and** `describe()` says *"N below the similarity floor"*, which is the
  sentence the field was always meant to produce.
- The decision file states plainly that the two new variants **break an exhaustive `match`**, and recommends
  `#[non_exhaustive]` at the next minor for this and the three sibling enums that have each paid the same cost.

**One criticism, recorded:** `want.to_lowercase()` is called **inside** the candidate loop, so a 10 000-asset library does
10 000 allocations per filtered query. The needle is fixed for the call and should be hoisted. It is not hoisted because
doing so changes the comparison's signature for a function with one caller — a trade recorded in `perf.md` rather than
made silently.

**Verdict: GREAT.**
