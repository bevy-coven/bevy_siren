# Iteration 84 — research: a second optional section, and what a sentinel would have cost

Completes [#21](https://github.com/bevy-coven/bevy_siren/issues/21)'s last criterion by implementing
`needs-decision/loudness-in-the-index.md`'s recommendation.

## Sources

| Source | The rule it reports |
|---|---|
| `needs-decision/loudness-in-the-index.md` | recommends option **B**: an optional `loudness` section, *"absence encoded as absence"* |
| ADR 0003 (`docs/adr/0003-acoustic-features.md`) | measured that `ffmpeg` reports **−70.0 LUFS** and `ebur128` reports **`-inf`** for a 0.2 s clip — **both are numbers, and one is also a real level** |
| `src/semantic/loudness.rs` | `integrated: Option<f32>`, plus `blocks` and `blocks_possible`, which separate *too short* from *silent* |
| iteration 83 (`8a323a1`) | the same optional-section shape, already implemented and proved for `provenance` |
| `docs/loop/log.md` | *"actionable and non-breaking, so it need not wait"* — the precedent for acting on such a recommendation |

## The technique, in the sources' terms

A gated loudness measurement has a state that is neither a level nor a silence: **too short to measure**. ADR 0003
measured three implementations and found three wrong answers for the same 0.2 s clip — `−70.0` (the floor presented as a
level), `-inf` (not comparable), and the crate's own `None`. A storage format that writes a number there destroys the
distinction the type was built to keep.

So the section writes `-` for an absent integrated loudness, and the parser reads `-` back as `None`. Everything else in
the record is a real measurement even for a clip too short to be gated: a 0.2 s click has a **true peak** and a
**sample peak**, so those are always written.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| absent integrated loudness | `-` | the decision file's option B |
| record layout | `<16 hex hash> <lufs\|-> <dbtp> <sample peak> <blocks> <blocks possible>` | this design |
| the two counts | written, not derived | `loudness.rs`: they distinguish *short* from *silent* |
| non-finite level | refused at parse | the crate's rule that a number which is not a number is not a measurement |
| section order | marker recognised before section parsing, so sections may appear in any order | measured — see below |

## What good sounds like

An index written yesterday loads today with `loudness.is_empty()`, and one written today with records returns them
**field for field**, including the `None` that means *too short*. A loudness and a provenance section coexist and neither
consumes the other's marker lines. And a record carrying a level that cannot be written back is **refused by name**
rather than silently dropped or repaired: the failure is loud, which is the crate's rule at every other layer.
