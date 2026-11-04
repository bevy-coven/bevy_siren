# Iteration 108 — plan: make a missing fixture fail

## Ranked

1. **`CueTable::problems`** — chosen. Iteration 107's recorded follow-up, the third of three, and measured: seven variants,
   six fixture rows, suite green.
2. `#28` — `serde_json`, absent from the graph, plus a `leaf.rs` row: a dependency decision.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` waits on the owner; this is the last of a pattern already found twice, and it is a real — if narrow
— hole: the test whose job is *every problem prints as a sentence naming its cue* never produced one of the seven kinds.

## Design

`tests/table.rs::problems_names_each_mistake`:

- a `kinds!` macro lists every `CueProblem` variant **once** and generates the exhaustive `match` (no wildcard) and the
  `KINDS` list together;
- the fixture gains a cue `inert` (`Repeat::AvoidRepeat { avoid: 0 }`) and the expected list a row for the seventh variant;
- the test asserts every listed kind is produced by some fixture cue;
- the `named` closure is **deleted**: its comment was false and its assertion was implied by `contains`.

No change to `src/`.

## Fields

None.

## Tests

- `problems_names_each_mistake`, extended as above.
- **Proved with two experiments on an added `Tenth` variant**: not listed → `E0004`; listed with no fixture →
  `no fixture cue produces ["Tenth"]`. Both reverted; `src/cue.rs` returned to its original content hash.
- **The control** is the state this iteration started in: `HEAD`'s test, seven variants, six rows, **green**.

## Carved off

Nothing from the item. The `#[non_exhaustive]` decision stays open; its file now records the one cost it had missed — the
`all` test crate honours the attribute, so the `kinds!` match would need a `_ =>` arm — and corrects a sentence of iteration
105's that claimed the test had been fixed when it had only been given an arm.
