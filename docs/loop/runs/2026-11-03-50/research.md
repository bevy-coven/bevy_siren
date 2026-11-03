# research.md — iteration 50: the master is the headroom trim, and two demonstration files stop clipping

## The item

Iteration 49 left backlog #3 as a question: **the shipped table clips** — `Effects` reaches `+6.0 dBFS` and `Music`
`+4.0 dBFS` — and I had written *"whether the example's buses should be trimmed is an art decision"*. It is not, once
the decision file is read.

`docs/loop/needs-decision/above-unity-gain.md` already answers the crate half of it, and its recommendation is **C, do
nothing**:

> A gain above unity is a **mixing decision**, not an authoring mistake … The master's own level
> (`SirenSettings::master_db`) is the tool for it, and a host that mixes by ear or by meter sees it.

As of iteration 49 the crate **has** that meter — `SirenMix::worst_peak_db`. What was missing was the arithmetic that
connects the two, and any statement in the guide that the master is the tool.

## Sources

**S1. `needs-decision/above-unity-gain.md`, the recommendation** — *"The master's own level (`SirenSettings::master_db`)
is the tool for it"*. The claim this iteration tests.

**S2. `SirenSettings::master_db`** — *"The master level, applied after every bus"*, default `0.0`. If S1 is true, it must
appear inside `worst_peak_db`; if it does not, S1 is advice the crate cannot honour.

**S3. `model::chain_db`** and `update_mix` — `base = chain_db(&[master, bus.gain_db])`, so the master is already in every
bus's dB. `worst_peak_db` reads `bus_db`, which is that chain.

**S4. `examples/render`, step 6** — `write_gain(&out, "mix.wav", &mixed, 1.0, sum_db(&gains))`: the **sum of ten cues'
gains** applied to one 0.5-amplitude tone. `sum_db` is +18.58 dB, so the file is `0.5 × 10^(18.58/20) ≈ 4.2×` full scale —
the `+0.2 dBFS` true peak `ebur128` reported.

**S5. `docs/research/siren.md`, `MIX_TAU_S` and the loudness note** — a game mix is mastered below a music master; the
skill's own number is that *"a game mix is usually mastered lower than −14 LUFS"*. Measured after this iteration's change:
**−18.6 LUFS, −6.1 dBFS true peak**.

## The technique

1. **`tests/mix.rs::the_master_is_inside_the_worst_peak`** — one voice at unity, master at `0`, `−6`, `−20` dB, and
   `worst_peak_db` reads `0`, `−6`, `−20`. S1's claim, measured.
2. **`examples/mixer`'s buses trimmed** to give the demonstration a decibel in hand: `Effects` `0.0 → −7.0` and
   `Music` `−2.0 → −7.1`, which is `2 × 10^(−7/20) = −0.98 dBFS` and `−1.08`. The panel's peak clause disappears because
   nothing reaches 0 dBFS any more — the correct signal.
3. **`examples/render`'s `mix.wav` written at `master_db`**, not at `sum_db` of every cue, and its report says which:
   *"all ten at once through nothing; this file is written at `master_db` +0 dB"*.
4. **The guide states the trim** — *"`master_db` is the trim, and it is inside the number"* — and why `problems()` stays
   silent about loudness.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the master's effect on `worst_peak_db` | 1:1 — −6 dB of master, −6 dBFS off the bound | S1, S3 — **measured** |
| `Effects` with two voices at 0 dB | `+6.02 dBFS` | iteration 49, measured |
| `Effects` at −7.0 | `−0.98 dBFS` | computed, then read off the panel as *no clause* |
| `Music` at −7.1 with two layers | `−1.08 dBFS` | computed |
| `mix.wav` before | `+0.2 dBFS` true peak, `−4.3 LUFS` | `ebur128`, measured |
| `mix.wav` after | **`−6.1 dBFS`, `−18.6 LUFS`** | `ebur128`, measured |
| a game mix's loudness | below −14 LUFS | the skill's own target |

## What good sounds like

A `Music` block with **no** peak clause — because the table has a decibel in hand — and a `mix.wav` at
`−18.6 LUFS / −6.1 dBFS` whose report line says what it is and what it is not. Two files that used to be written past
full scale, and a guide sentence that names the field to turn.
