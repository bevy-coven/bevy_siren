# Iteration 85 — review

## Pass 1 — Function: is loudness now produced by a build?

**Critique.** The gap was exact and iteration 84 named it: `loudness::measure` was tested since 77 and **nothing called
it**. This iteration calls it, and the test asserts three things a store-only implementation would fail:

| Claim | Assertion | Why it is not vacuous |
|---|---|---|
| the build measures | `index.loudness.len() == 1` | a store-only build has zero |
| it measures the **right number** | ≈ **−9.0 LUFS** ±0.5 for a 0.5-amplitude 1 kHz tone | the anchor is the **−23.0** a 0.1-amplitude tone gives (`20·log10(5)` = 13.98 dB), measured in iteration 77 |
| it measured every file | `unmeasured == 0` | and `describe()` prints the count when it is not |

**Critique of the sharing.** Measuring **inside** `embed_one`'s existing buffer is the point: a second decode would
double the cost of the operation the whole feature exists to make cheap. One decode, two consumers, and neither keeps
the buffer.

**Critique of the failure policy.** A failed measurement is **not** a failed file — the vector is the index's reason to
exist — but it is **counted**, because a run that measured nothing must not look complete. That is the same rule as a
refusal being silent: the failure exists to be read.

**Verdict: GREAT.**

## Pass 2 — Zoom: the block count that was wrong in three places

**Critique.** This is the pass that matters, because the defect is not in the new code — it is in **shipped
documentation**, found only because the new test asserted an exact number.

`loudness.rs` said a 3 s clip has **7** gating blocks. Its own test said *"3 s at 75% overlap is seven blocks"*. Both are
wrong:

    1 + (144000 − 19200) / 4800 = 27

7 is the **non-overlapping** count. A 75 %-overlapping block advances 100 ms, so 3 s gives 27. The existing assertion was
`blocks >= 7`, **which passes on 27** — a lower bound cannot detect a wrong sentence, only a wrong magnitude.

Fixed in three places, and the doc table now says **1 s** where 7 is the correct number, so the contrast the table makes
(1 s of silence has 7 blocks and 0 above the gate; 0.2 s has 0 possible) is both true and still the point.

**A second, smaller find**: the new test's embedder closure took an unused `buffer`, which `clippy -D warnings` caught.
Prefixed, as the lint suggests.

**Verdict: GREAT.** A false claim in user-facing doc text was found and corrected, and the mechanism that hid it (a
lower bound) is replaced by an exact value with its arithmetic in the message.

## Pass 3 — Fresh eyes: what does a caller see?

**Critique.** Nothing changes for a consumer of the *built* API except one new field on `Built`, which the crate alone
constructs. What changes is that `build` now returns an index whose `loudness` map is **populated** rather than empty,
so a search result's loudness is real without a second pass — which is what `#21` asked for and what iteration 84 could
only store.

`describe()` gains a phrase only when the number is non-zero, so a healthy run reads exactly as it did before:
`"1000 indexed"`, not `"1000 indexed, 0 unmeasured"`.

**The honest limit**, stated rather than glossed: **provenance is still not wired.** `Sidecar::read` exists and nothing
calls it, for the same reason `measure` was unwired until now — it needs the scan's path→hash map joined to the
sidecar's name→record map, which is a larger change than this one and is the next item.

**Verdict: GREAT.**
