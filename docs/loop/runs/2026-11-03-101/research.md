# Iteration 101 — research: a dry/wet estimate, and how much it can be trusted

## The item

`#23`. Its own note is the one that shaped this iteration: *"Be candid: this is the feature most likely to be a number
that looks precise and means little. If the discrimination measurement is poor, say so and recommend dropping it from the
default weights rather than shipping it because it exists."*

## Sources

| Source | What it reports |
|---|---|
| **ISO 3382-1** — *Acoustics: Measurement of room acoustic parameters* | **C50**, the clarity index: `10·log₁₀(early/late)`, boundary **50 ms**. The standard's definition of "how much of the sound is direct" |
| Mack, Deng & Habets, *Single-Channel Blind Direct-to-Reverberation Ratio Estimation Using Masking* — **DOI 10.21437/interspeech.2020-2171** | the learned approach. Downloaded (9/10 parts); its method is a masking network, which is exactly the ML dependency the acoustic features exist to avoid |
| Pérez-López, Politis & Gómez, *Blind reverberation time estimation from ambisonic recordings* — **DOI 10.1109/mmsp48831.2020.9287128** | T60 from ambisonic recordings; unavailable (0 bytes) — recorded rather than cited as read |
| *Speaker Distance Estimation in Enclosures from Single-Channel Audio* — **10.48550/arxiv.2403.17514** | quotes the DRR definition: *"the balance between direct speech energy versus early and late reverberant energy"*, and notes reverberation intensity is *"roughly independent of its distance"* while the direct sound is not. This is the physical basis for reading a ratio rather than an absolute level |
| *Dependence on Early and Late Reverberation of Single-Channel Speaker Distance* — **10.48550/arxiv.2605.07694** | evaluates against **DRR, C50 and T60**, and finds *"estimation accuracy improves with stronger early energy and degrades in highly reverberant environments"* — a measured warning that the wet end is where any envelope estimate is least reliable |

## The technique

`C50` adapted to a single channel with no impulse response:

```text
C50 = 10 · log₁₀ ( energy in the 50 ms after the peak  /  energy in the next 500 ms )   [dB]
```

**The adaptation, stated rather than glossed**: ISO 3382 defines C50 on a measured impulse response, where the direct
sound's arrival is known because the excitation is. A game asset is not an impulse response, so the **loudest frame is
the direct-sound proxy** and the 50 ms after it is *early*. That is an approximation. The learned alternative exists
(Mack et al.) and is rejected on the same ground every other feature here rejects a model: it is a dependency the crate
does not take.

Frames come from `analysis::spectra`, the shared pass iteration 95 built, so this costs **no second transform**.

## Parameters

| parameter | value | source |
|---|---|---|
| `early_window_s` | **0.05 s** | **ISO 3382's C50** |
| `late_window_s` | **0.5 s** | **design default** — long enough to contain a tail up to about RT60 0.4 s, short enough that a quiet passage after a sound is not counted as its reverb |
| `Reverb::CEILING_DB` | **60 dB** | **design default** — 1000:1 in energy, past which the distinction stops being audible and any further number is arithmetic about a zero |

## The measurement, and what it is worth

```
REVERB DISCRIMINATION: 12/12 pairs separated (100 %); dry ≥ 60.0 dB, wet ≤ 1.9 dB; bands disjoint
```

**Twelve dry clips, each reverberated by convolution with a synthetic exponentially-decaying tail (RT60 0.9–1.45 s).**

**What that proves**: the direction is right, the bands do not overlap, and the measure is not dominated by the
perceptually wrong cue — on material whose reverb was added by a formula.

**What it does not prove, and this is the honest part**: a synthetic tail is a clean exponential with no modal
structure, no early reflections and no absorptive differences. A real studio recording and a real cave differ in more
ways than decay rate — a cave's noise floor is higher, its early reflections are sparse, and a studio recording may have
any reverb at all baked in from the room it was made in. **Real recordings have not been measured**, and
`DISCRIMINATION_MEASURED` says so in the constant's own text.

## What good sounds like

A recording made in a treated room should read **above +20 dB**; a cave or a cathedral **below 0 dB**. A footstep
0.1 s long should report *no measure*, not a confident number.

## The recommendation the issue asks for

**Keep it out of the default weights.** `Weights` has four signals and this adds a fifth only once a labelled set of
**real** recordings says it separates them. The measure is built, tested and stored so that measurement can be made;
shipping it into the ensemble on synthetic evidence would be the exact mistake `#23` warns against.
