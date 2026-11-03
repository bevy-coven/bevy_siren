# research.md — iteration 47: a duck that starts on the bus it lowers quietens every voice on it

## The item

Iteration 45's backlog listed a **self-duck** — `Duck::new("Effects", "Effects")` — as an open question, and iteration
46's first probe of it produced a *different* finding (`Duck::new`'s zero hold), so the question was never settled.

**Measured this iteration** (`tests/mix.rs::a_duck_whose_source_is_its_target_lowers_the_bus_that_fired_it`, with the
hold and fade the rule needs to be audible at all):

| | `SirenMix::ducked["Effects"]` | `bus_db("Effects")` |
|---|---|---|
| no voice on the bus | `0.0` | 0 dB |
| a voice starts | **`-6.0`** | **mixed 6 dB lower** |

The mechanism is structural, not accidental: `fire_all` matches rules on **`source`** and `duck_db` matches them on
**`target`**. When those are one bus, the voice that fired the rule is multiplied by it — a footstep pulling down every
other effect for the rule's whole envelope, and itself with them.

## Sources

**S1. `Duck::source`'s doc** — *"The bus whose voices start the duck"*, and `target` — *"The bus lowered while they
play"*. Two different sentences about two different buses; nothing says they may coincide.

**S2. `DuckClock::fire_all`** — *"Fires every rule whose `source` is `source`"*, and `duck_db` — *"The strongest duck on
`bus`"*, filtered on `duck.target == bus`. The two halves that make a self-duck self-apply.

**S3. `docs/guide/buses.md`** — *"two ducks do not stack: the deeper one wins, because a bus already at −8 dB is not
lowered another 4 dB"*. The guide reasons about two **rules** on one bus, never about a rule whose two ends are the same.

**S4. `docs/loop/needs-decision/README.md`** — *"only the public API, the message types and the definitions-file schema
come here"*. A `problems()` sentence is `Vec<String>`, additive, breaks no `match` — so it is **not** a decision. This
iteration wrote `self-duck.md`, then deleted it: the crate's own rule says decide it here.

**S5. Gregory §14.6.3** (vendored) — ducking is *"a group lowering another group"*. A group lowering itself is a
compressor, a different device with different parameters; a table field named `source`/`target` is not that.

## The technique

An eleventh sentence in `SirenSettings::problems`:

```
the duck on `Effects` starts on the bus it lowers, so every voice on it quietens itself
```

Phrased as a description of the mechanism, not a verdict — a project that meant a self-sidechain reads one true sentence
and moves on.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a rule is fired by a voice on `source` | — | S1, S2 |
| a rule lowers `target` | — | S1, S2 |
| a self-duck's depth on the frame its own voice starts | **−6.0 dB** (authored −6.0) | measured |
| the bus it mixes | **the same 6 dB lower** | measured |
| the sentence's length | 78 chars — one line in the panel's column | measured on the capture |

## What good sounds like

A `Problems` block containing `the duck on \`Effects\` starts on the bus it lowers, so every voice on it quietens
itself` for a table with one, and **nothing** for the crate's `default_ducks()` (Dialogue → Music, Dialogue → Effects),
whose two ends always differ.
