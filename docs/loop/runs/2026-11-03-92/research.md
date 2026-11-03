# Iteration 92 — research: key detection, and an ADR's assumption disproved

The item is [#25](https://github.com/bevy-coven/bevy_siren/issues/25). ADR 0003 **declared key out of scope** on the
grounds that a detector would be *"wrong 40 % of the time"* and would make `#27` penalise the right track. That 40 % was
never measured. This iteration measures it.

## Sources

| Source | The rule or number it reports |
|---|---|
| Krumhansl & Kessler (1982), *Psychological Review* 89(4) | the **twelve probe-tone ratings** for major and minor scales, the profiles every Krumhansl-Schmuckler detector uses |
| Krumhansl (1990), *Cognitive Foundations of Musical Pitch* | the correlation method: rotate each profile against a chroma vector, take the highest Pearson `r` |
| ADR 0003 (`docs/adr/0003-acoustic-features.md`) | key *"out of scope for the Rust path"*, on the assumption of a **40 % error rate** and no reported Rust accuracy |
| `#25`'s criteria | ≥20 tracks with known keys; exact-match **and** fifth-compatible recorded separately; an atonal asset reports no key; the compatibility table is a **named type** |
| `src/semantic/mel.rs` | `fft_in_place`, the radix-2 transform tested to 0.0545 dB against CLAP |

## The technique, in the sources' terms

Audio → **chroma vector** (twelve pitch classes by energy) → 24 rotations of the major and minor profiles → the highest
**Pearson correlation** wins. That is Krumhansl-Schmuckler, and it is the standard method because the profiles encode
what listeners actually hear as stable in a key.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| major profile | `[6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88]` | Krumhansl-Kessler |
| minor profile | `[6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17]` | Krumhansl-Kessler |
| chroma window | **8192 points** — 5.86 Hz/bin, 5.0 bins between B4 and C5 | measured; see below |
| confidence floor | **0.15** | **design default**, measured against three clips |
| corpus | **24 keys** — every tonic, both modes, I–IV–V–I | `#25` asks for ≥20 |

## What good sounds like — and the five defects between the first draft and the number

**Measured result: 100 % exact, 100 % fifth-compatible, on all 24 keys.** Getting there took five corrections, and every
one was found by printing a number rather than by reasoning.

1. **A 1024-point window cannot see a pitch.** At 48 kHz one bin is **46.875 Hz — 2.85 semitones at C4** — and a pure C4
   peaks in bin 6 (**281 Hz**), which the first version read as C#. **4096 points** fixed the lowest octave.
2. **But 4096 is not enough at the top.** B4 (493.9) and C5 (523.3) are 29 Hz apart and a Hann main lobe is four bins
   wide: at 11.72 Hz/bin they are **2.5 bins** apart and merge, which is why B tied the tonic. **8192 points** gives
   **5.0 bins** and separates them.
3. **Band edges were half a semitone out of phase.** A note `n` spans MIDI `[n − 0.5, n + 0.5)`, not `[n, n + 1)`;
   comparing against `note` and `note + 1` split C4's own partial between **B and C**. Measured on a pure C major scale:
   B scored **10.71** against C's **5.80**. This was the defect that produced every wrong answer.
4. **Log-magnitude compression flattened the vector** to the point where all twelve classes fell between 3.96 and 5.41 —
   correlation needs contrast. Reverted to a linear magnitude.
5. **The fixture was musically wrong.** A tonic pedal held under the **V chord** puts C against B — a **minor second**,
   the most dissonant interval in the scale — so the passage genuinely contained as much B as C. The detector's "no key"
   was *correct*. The pedal is now the chord's own root an octave down.

**And the optionality threshold was measured, not chosen:** a progression scores **0.241**, a single impact **0.067**, white
noise **0.030**. `0.05` sat above noise and *below* the impact, so a single hit reported a confident `D# minor`. It is
**0.15**.
