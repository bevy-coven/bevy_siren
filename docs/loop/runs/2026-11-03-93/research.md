# Iteration 93 — research: wiring tempo and key, and the cost of a fourth pass

The item is iteration 92's first leftover: **tempo and key are built and tested, and `build` never calls them.** One schema
change carries both, which is why they waited.

## Sources

| Source | The rule it reports |
|---|---|
| `src/semantic/index.rs` | three optional sections already exist (`provenance`, `loudness`, `spectral`), each written only when non-empty and each inside the checksummed body |
| `src/semantic/loudness.rs`, iteration 77 | `integrated: Option<f32>` — an absent measurement is a **state**, not a sentinel |
| `src/semantic/tempo.rs`, iteration 91 | `bpm: Option<f32>`, `confidence`, `octave_ambiguous` |
| `src/semantic/key.rs`, iteration 92 | `Estimate { key, confidence }`; `Key::absent()` for an atonal asset |
| `src/semantic/indexer.rs` | `embed_measured` already decodes, downmixes and resamples **once** and shares that buffer with loudness, spectral, and now tempo and key |
| `runs/2026-11-03-88` | the decode baseline these features are measured against: **2 411–2 724 clips/s** before any of them |

## The technique, in the sources' terms

A fourth and fifth pass over a buffer that is already in hand is cheap *conceptually* and expensive *in fact*, and the
distinction is the whole result. `embed_measured` shares the decode — so no file is read twice — but tempo runs its own
autocorrelation and key runs its own **8192-point** transform, and neither shares arithmetic with the mel path.

The storage follows the rule the last two sections established:

- **A record with an absent value is written**, because `bpm: None` (a non-rhythmic asset) and *no record at all* (an
  asset too short to measure) are different facts, and a filter reads the first as *unjudged* and the second as *unknown*.
- **An empty section is not written**, so every index built before this iteration is byte-identical and still loads.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| tempo record | `<hash> <bpm\|-> <confidence> <octave 0\|1>` | this design |
| key record | `<hash> <tonic\|-> <major 0\|1> <confidence>` | this design |
| presence byte (binary) | one per optional value, as loudness and spectral already do | the established shape |
| decode before | **1 150 clips/s** (iteration 89's baseline) | measured |
| decode after | **420–570 clips/s** | measured, three runs |

## What good sounds like — and the cost that is the headline

A build of a metronomic click track stores **120 BPM ±2** with no caller supplying it, and a click train's key record
says **no key** — a record, not a gap. A 0.1 s clip stores a **tempo record with `bpm: None`** (4 800 samples is enough
for tempo's two 1 024-sample frames) and **no key record** (8 192 samples is not). Three distinct states, all preserved
through both formats.

**And the decoding cost more than halved.** Measured at 200 clips: **419.8** and **568.8 clips/s** across two runs,
against **1 150** before this iteration. Two extra passes over every file — one autocorrelation over ~400 onset frames,
one 8192-point transform per chroma frame — is the whole of it.

That is a **2.0–2.7× regression** on the scan. It is the case this iteration targeted, so it is a trade rather than a
leak, and 5 000 clips still costs **13 s** of decode against the ~3 minutes the audio tower itself would take. But it is
the third iteration in a row where a feature added a pass, and the next fix is obvious and unbuilt: **one shared
spectral pass** producing mel, centroid, chroma and onset together.
