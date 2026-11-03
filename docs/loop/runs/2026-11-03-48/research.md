# research.md — iteration 48: a bus cap above the table's budget does nothing, and nothing said so

## The item

`SirenSettings::bus_voice_limit` is `min(cap, total)`. A bus asking for **more** voices than the table allows is
therefore silently given the table's — `BusLevel::new("Effects").max_voices(16)` under `max_voices: 4` mixes **four**.
Measured last iteration and again here with a test:
`tests/mix.rs::a_bus_asking_for_more_than_the_table_is_clamped_and_reported`.

Two documents stop short of saying so:

- **`BusLevel::max_voices`'s doc**: *"The most voices this bus may sound; `0` is the table's own `max_voices`."* It
  names the **sentinel** and not the **clamp** — a reader learns what `0` means and nothing about `16` under a table of 4.
- **`docs/guide/buses.md`**: *"a bus's own `max_voices` caps it further"*. True, and silent about a number that is
  larger: an author who writes 16 for effects and a table of 4 has raised nothing, and the guide's sentence reads as if
  they had.

A number an author writes and no one discovers is the same class as iterations 46 and 47: the report is where this
crate says so.

## Sources

**S1. `bus_voice_limit`** — `Some(0) | None => total`, `Some(cap) => cap.min(total)`. The clamp, in one expression.

**S2. `BusLevel::max_voices`'s doc** — quoted above: sentinel, not clamp.

**S3. `docs/guide/buses.md`** — *"a bus's own `max_voices` caps it further (`0` is the table's)"*, and *"all three are
**budgets on plays**, not standing counts"*. The three-budget framing is right; the clamp is missing.

**S4. `Duck`'s sibling precedent** — iterations 46 and 47 added sentences for a rule with no hold (an inert value) and a
rule that lowers its own bus (a surprising one). Both were **numbers that do not do what the words say**, and both were
reported rather than enforced.

**S5. `docs/loop/needs-decision/README.md`** — *"only the public API, the message types and the definitions-file schema
come here"*. `problems()` is `Vec<String>`: additive, no `match` breaks. Not a decision.

## The technique

A twelfth sentence in `SirenSettings::problems`:

```
bus `Effects` asks for 16 voices but the table's max_voices is 4, so it mixes 4
```

Guard: `bus.max_voices > 0 && bus.max_voices > self.max_voices`. The `> 0` keeps the sentinel out (a bus with `0` is
asking for the table's, which is exactly what it gets), and `>` rather than `>=` keeps a cap **equal** to the table's
silent — 4 of 4 asks for what it gets.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the clamp | `min(cap, total)` | S1 |
| a bus's cap larger than the table's | the table's | S1 |
| a bus's `0` | the table's, and correctly so | S2 |
| a cap **equal** to the table's | honoured exactly — not a fault | **design default** |
| the sentence's width | 71 chars — one line in the panel's column | measured on the capture |

## What good sounds like

A `Problems` block reading `bus \`Effects\` asks for 16 voices but the table's max_voices is 4, so it mixes 4`, with
every bus row under it agreeing (`Effects 2 / 4`). Nothing at all for the crate's default table, whose buses ask for
`0` and `2` under `12`.
