# plan.md — iteration 48: a bus cap above the table's budget is reported

## The pick, and why it beats the runner-up

**Pick:** `bus_voice_limit` is `min(cap, total)`, so a bus asking for more voices than the table allows silently gets
the table's — and both documents that describe the field (`BusLevel::max_voices`'s doc names only the `0` sentinel; the
guide says the bus *"caps it further"*) leave a reader believing a larger number raised it.

Beats the runner-up (`MissingSound` capture): that shows something already working; this is **a number an author writes
and never discovers**, the same class as iterations 46 and 47.

## The blocked item

**Case 31** (a non-finite cue `gain_db` reported by `problems()`) — still blocked on
`needs-decision/bad-gain-problem.md`, a public enum. Nothing built from it.

## Why this needs no decision file

`SirenSettings::problems` returns **`Vec<String>`** — additive, no consumer's `match` breaks. `needs-decision/README.md`
says only the public API, the message types and the schema come there. Iterations 46 and 47 relied on the same fact.

## The design

1. **A twelfth sentence**: *"bus `Effects` asks for 16 voices but the table's max_voices is 4, so it mixes 4"*. Guard
   `max_voices > 0 && max_voices > self.max_voices` — `> 0` keeps the sentinel out, `>` keeps a cap **equal** to the
   table's silent.
2. **The doc's list gains a twelfth bullet**; "Twelve checks".
3. **`docs/guide/buses.md`** now says a number above the table's does nothing.
4. **`tests/mix.rs::a_bus_asking_for_more_than_the_table_is_clamped_and_reported`** — rewritten from last iteration's
   silent-clamp test to assert the sentence, with its two must-stay-quiet cases (`0`, and a cap equal to the table's).
5. **`problems_names_every_mistake`** gets a dedicated table — the big one has `max_voices: 0`, which hides the clamp
   behind the "no sound can ever play" sentence.
6. **`SIREN_MIX=overbudget`** gives the fault its own table, because `broken` already fills the block's three-sentence
   cap and a fourth fault would be the invisible `and 1 more`.

## Fields

None. `BusLevel::max_voices` already exists.

## Tests

- **`a_bus_asking_for_more_than_the_table_is_clamped_and_reported`** — the clamp, the sentence, the `0` sentinel and a
  cap equal to the table's. Mutation-checked (`if false` reddens it).
- **`problems_names_every_mistake`** — the sentence in the indexed list.

## The carve

- **Not** an error: a table whose bus asks for 16 under 4 still plays four voices. The number is reported, not refused —
  the crate's line is that a *table* fault is a sentence.
- **Not** a `problems()` sentence for a **cue** whose `max_instances` exceeds its bus's cap: the bus steals rather than
  refusing, so that table is not broken. (Checked and recorded in iteration 42's carve.)
