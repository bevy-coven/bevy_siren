# Iteration 92 — review

## Pass 1 — Function: does key meet #25, and was ADR 0003 right?

**Critique.** Three criteria, and all three are tests:

| # | Criterion | Test | Why it is not vacuous |
|---|---|---|---|
| 1 | accuracy on ≥20 known keys, **exact and fifth separately** | `the_corpus_accuracy_is_recorded` | **24 keys**, every tonic in both modes, and both percentages printed |
| 2 | an atonal asset reports **no** key | three tests | a single impact, white noise, **and** silence — three different ways to be atonal |
| 3 | the compatibility table is a **named serde type** | `the_compatibility_table_penalises_a_clash_and_never_an_unknown` | it is `Compatibility`, with `parallel_penalty` and `clash_penalty` as fields |

**And the ADR question.** ADR 0003 excluded key on the assumption of a **40 % error rate**, never measured. The measured
rate is **0 % on 24 synthetic keys**. That does not mean 100 % on a game's assets — it means the exclusion was argued from
a number nobody had, and the number is now in the record. `needs-decision/key-detection.md` carries what to do with it.

**Critique of the method.** Krumhansl-Schmuckler with the Krumhansl-Kessler profiles is the standard, and the twelve
numbers are **vendored in a public struct** rather than cited by name: a key detector *is* those numbers, and a reader
must be able to see them. It is deliberately **not** a serde type like `Weights` — a project that edited them would be
detecting a different key system, which is research rather than taste.

**Verdict: GREAT.**

## Pass 2 — Zoom: five defects, all found by printing a number

**Critique.** Every one of these would have shipped as a plausible-looking detector:

1. **1024-point window.** 46.875 Hz/bin is **2.85 semitones at C4** — no bin mapping can recover a pitch from a band that
   wide. A pure C4 peaked at bin 6 (**281 Hz**) and read as C#.
2. **4096 is not enough either.** B4 and C5 are 29 Hz apart; at 11.72 Hz/bin that is **2.5 bins** against a four-bin Hann
   main lobe, so they merged and B tied the tonic. **8192** = 5.0 bins.
3. **Half-semitone band offset.** Note `n` spans `[n − 0.5, n + 0.5)`; comparing `note`/`note + 1` split C4's partial
   between **B and C**. Measured: B **10.71** against C **5.80** on a pure C major scale. **This was the defect that
   produced every wrong answer**, and it survived two window changes because the window was also wrong.
4. **Log compression flattened the chroma** to 3.96–5.41 across all twelve classes. Correlation needs contrast; reverted.
5. **The fixture was musically wrong** — a tonic pedal under the V chord is a **minor second against its third**. The
   passage really did hold as much B as C, so the detector's "no key" was *correct* and my test was asserting the
   detector should be wrong.

**The general lesson, sixth in this series**: print the number the code computes. Three of these five were invisible in
the output — the function returned a key, just the wrong one — and only dumping the chroma showed the classes were
misplaced.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does a caller see?

**Critique.** `Key { tonic: Option<u8>, major: bool }` and `Estimate { key, confidence }`. An atonal asset is
`Key::absent()`, which prints `no key` — and `#27`'s filter already treats an absent feature as *unjudged*, so the two
features compose without special-casing.

The compatibility table is one call:

```rust
let table = Compatibility::default();
let multiplier = table.multiplier(a.relation(&b));   // 1.0, 0.85, or 0.5 — and 1.0 for an unknown
```

`Relation` names the four cases a person would name — same, parallel, a fifth, clash — and `multiplier(None)` is `1.0`,
which is `#25`'s rule that an unknown key never penalises.

**The honest limit, stated in the module doc**: **100 % is on synthetic tonal material**, which is what a detector should
find easy and what no asset library resembles. Claiming it as a general accuracy would be the same error as ADR 0003's
unmeasured 40 %, in the flattering direction.

**Verdict: GREAT.**
