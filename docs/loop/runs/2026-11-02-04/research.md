# research.md — iteration 4: a crossfade that takes the time it was given

## The item

`MusicLayers::fade_s` is accepted by the builder, stored in `Pending`, and **read by nothing**. `move_layers` eases
every layer toward its target with the constant `MIX_TAU_S`:

```rust
let next = approach(layer.db, layer.target_db, MIX_TAU_S, dt);
```

`MIX_TAU_S` is 0.05 s, so a layer settles in about 0.15 s — three time constants. A project that writes

```rust
layers.write(MusicLayers::new(vec![("combat".to_owned(), -3.0)]).next_bar().fade_s(0.8));
```

gets a 150 ms entry instead of an 800 ms one, and nothing says so: no log line, no refusal, no field left unset. It is
the same class as iterations 2 and 3 — a number a person wrote whose sentence the code does not honour — and it is
rank 2 in the loop's order ("music transitions and layering … fades") but **rank 1 in practice**, because the number is
already public, already documented as *"how long the move takes once it starts"*, and already ignored.

`docs/loop/needs-decision/stinger-release.md`'s closing note predicted this as iteration 4's first item. It is.

## Sources

1. **The crate's own rule R5** (`docs/research/siren.md`): *"A score is layers of the same performance; a layer at
   `SILENT_DB` stays in the mix so every layer is phase-aligned. A change is quantised to the next beat or bar from
   tempo alone."* Its citation is Berndt-style adaptive scoring (Khan et al. 2023, 10.48550/arXiv.2303.15734, read in
   iteration 0): vertical layering moves **gains**, and the movement is the audible part of the transition. A gain
   that arrives 5× faster than authored is a different transition, not a faster one.

2. **The crate's own envelope form** — `model::approach(current, target, tau, dt)`, pinned by
   `tests/model.rs::approach_moves_one_time_constant_at_a_time`: one time constant moves 63.2 % of the way, and
   `tau <= 0` or `dt <= 0` returns the target (a step when no time is given). Every existing fade in the crate —
   buses, stops, duck releases — is built from this one function, so a layer fade built from it too is one rule, not
   a second.

3. **`src/mix.rs`'s documented convention for a fade duration.** `Duck::fade_s` is *the time the recovery takes*, and
   `active_duck_db` interpolates **linearly** across it, reaching exactly the target at `hold + fade`. That is the
   crate's meaning of a duration: the time to arrive, not a decay constant. `MusicLayers::fade_s`'s doc says the same
   thing — *"How long the move takes once it starts"*.

4. **Bevy 0.20 / rodio 0.22.2, vendored** — the only two things this change touches at the edge: `Time<Real>` for the
   frame's `dt` (`bevy_time`, already the clock every other fade uses) and `AudioSink::set_volume` /
   `AudioOutput::set_voice_db`, which `update_mix` already drives. No new API, no new dependency.

5. **Literature server unavailable, again, and recorded rather than worked around.** Every
   `hs distill search` returned `Embedding error: gpu busy: 4736 MB free < 5000 MB required to load bge-m3` (a 14.8 GB
   VLLM engine holds the card; not this crate's process). The metadata APIs (`hs paper search`, OpenAlex/CrossRef) do
   answer, but returned **no abstract** for the adaptive-scoring papers, and nothing with a crossfade *duration*: the
   closest hits were on tone duration and intonation, not transition length. So this iteration does not claim a
   measured crossfade time from a paper. The number it ships is the **crate's own existing default**, and the
   conversion from "time to arrive" to a time constant is derived from `approach`'s own definition below.

## The technique

A linear arrival over `fade_s` is what `active_duck_db` does and what the field's sentence says. `approach` is
exponential, so the crate needs one **derivation**, not a new curve:

> `approach` reaches `1 − e^(−t/τ)` of the way after `t`. For that to be within one part in a thousand of the target
> at `t = fade_s` — the crate's own `0.01 dB` settle threshold, expressed as a fraction of a move of up to 80 dB —
> `e^(−fade_s/τ) ≤ 1/1000`, so **`τ = fade_s / ln(1000) ≈ fade_s / 6.9078`**.

That keeps one writer (`update_mix`), one curve (`approach`), and makes `fade_s` mean what it says: after `fade_s`
seconds the layer is at its target for every practical purpose, and the `0.01 dB` snap in `move_layers` finishes it.
`fade_s <= 0` keeps today's `MIX_TAU_S`, so the builder's default — *"`0` is the crate's `MIX_TAU_S` easing, which is
what a layer wants"* — stays exactly true.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| default layer easing | `MIX_TAU_S` = 0.05 s, ~150 ms to settle | `MusicLayers::fade_s`'s own doc: *"`0` is the crate's `MIX_TAU_S` easing, which is what a layer wants"* — unchanged |
| a fade's meaning | the time to arrive | `Duck::fade_s` + `active_duck_db` (crate convention) |
| `approach`'s settle | 63.2 % per τ | `tests/model.rs`, crate rule |
| time constant for an arrival in `fade_s` | `fade_s / ln(1000)` | derived here from `approach`'s definition; **design default**, since no source states a crossfade length |
| settle threshold | 0.01 dB | `move_layers`'s existing snap; `update_mix` uses the same 0.01 dB |
| a music crossfade a game asks for | 0.3–2 s, typically one bar | **design default**, and the reason the field is a field: the right number is the project's, and the crate's job is to spend it |

## What good sounds like

A layer brought in on the bar with `fade_s(0.8)` **should take most of a bar to arrive**, not a blink. The table is
the layer's gain, simulated at 60 fps with the two time constants (the crate's `approach`, `target = −9 dB`):

| time after the bar | today (τ = 0.05 s) | with `fade_s(0.8)` (τ = 0.1158 s) |
|---|---|---|
| 0.0 s | −80.0 dB | −80.0 dB |
| 0.2 s | **−10.3 dB** | −21.6 dB |
| 0.4 s | −9.0 dB (settled) | −11.2 dB |
| 0.6 s | −9.0 dB | −9.4 dB |
| 0.8 s | −9.0 dB | **−9.07 dB** → snaps to the target |

**A fraction-of-range figure would be misleading here and the table says why.** At 0.2 s the authored fade has covered
82 % of the *distance* from −80 to −9 dB — and is still at −21.6 dB, because a gain move near silence covers most of
its dB range in the last few percent of its amplitude. That is `approach` acting on dB, not a defect; the perceptual
shape is a fast rise out of silence and a long settle, which is exactly what a crossfade should sound like. What the
fix guarantees is the thing the field promises: **the layer reaches its target within `fade_s`, not within 150 ms.**

A reviewer can judge this without hearing it: the layer's gain plotted against time **keeps moving across the authored
duration** instead of hitting the target by frame 24 and flat-lining, and `MusicState::moving()` counts the layer as
moving for `fade_s` rather than for 150 ms. The plot in `evidence/plots/` draws both curves, so a later change
that reintroduces the old one shows as the two converging.
