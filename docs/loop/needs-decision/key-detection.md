# Key detection ships — but the 100 % is on synthetic material

- **Issue:** [#25](https://github.com/bevy-coven/bevy_siren/issues/25)
- **Status:** records a measurement and a recommendation; the detector is **built and tested**

## What ADR 0003 said

ADR 0003 excluded key from the Rust path because *"no pure-Rust implementation reports a usable accuracy figure"* and a
detector *"wrong 40 % of the time"* would make `#27` penalise the right track.

**The 40 % was an assumption.** No measurement was cited because none had been taken.

## What was measured

`semantic::key` implements Krumhansl-Schmuckler with the Krumhansl-Kessler profiles — the standard method — and on a
corpus of **24 keys** (every tonic, both modes, each a I–IV–V–I progression with the chord roots in the bass):

| | result |
|---|---|
| exact match | **24 / 24 — 100 %** |
| fifth-compatible | **24 / 24 — 100 %** |
| atonal assets refused (an impact, white noise, silence) | **all three** |

## What that does *not* mean

**Synthetic tonal material is the easy case.** A game's library is loops, stingers, ambience beds and one-shots: a
four-bar orchestral loop may detect well, a 0.8 s UI blip will not, and a drone with no tonal centre must report nothing
(which it does). The 100 % is a **ceiling on clean material**, not an expectation for a real library.

`#25` asks for the accuracy to be *recorded*; it is. A figure on real assets needs `V-01`'s labelled set.

## Options

### A. Ship the detector, with the confidence as the gate (recommended)

`Key::absent()` for anything under confidence **0.15**, and `Compatibility::multiplier(None) == 1.0`.

- **Benefit:** a filter that helps on tonal material and is **inert** on everything else. No real asset is penalised for
  a key nobody could hear.
- **Cost:** a wrong key above 0.15 will penalise a neighbour that actually fits. Measured, the confidence gap between a
  progression (0.241) and an impact (0.067) is wide *on synthetic material*; on real loops it may not be.

### B. Ship it, and have `#27`'s key rule **penalise only on a `Clash`**

- **Benefit:** the false-positive cost is bounded by `clash_penalty` (0.5), never an exclusion.
- **Cost:** a wrong key still moves a real match down by half.

### C. Do not ship; leave the key rule unjudged

- **Benefit:** nothing can be penalised for a key that is wrong.
- **Cost:** `#25` closes without a filter, and a library of tonal music gets no benefit from a feature that works on it.

## Recommendation

**A, with B's restraint** — which is what is built: the detector reports, the table penalises at most 0.5, and an
absent key multiplies by 1.0. That is the shape `#27`'s own criteria require (*"a feature that is unknown never excludes
an asset"*), and the one thing this measurement changed is that key is no longer *always* unknown.

## What would overturn it

A measurement on `V-01`'s labelled set showing that a wrong key above 0.15 is common enough to move right answers out of
the top *k*. If that lands, option C becomes correct and the `Key` type stays — the store and the type are independent of
whether the rule reads them.
