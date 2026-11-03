# plan.md — iteration 47: a self-duck is reported, and the loop stops asking

## The pick, and why it beats the runner-up

**Pick:** a duck whose `source` and `target` are the same bus. Measured: it lowers the bus that fired it — `-6.0 dB` on
the frame its own voice starts, and the mixed level with it. It is legal, undocumented as a fault, and **invisible in a
nine→ten-sentence report**. Beats the runner-up (a `MissingSound` capture) because that is a picture of something that
works and this is a rule that quietly quietens a whole bus.

## Why this is not a decision file, though I wrote one first

`docs/loop/needs-decision/README.md`: *"only the public API, the message types and the definitions-file schema come
here."* `SirenSettings::problems` returns **`Vec<String>`** — additive, no consumer's `match` breaks — so by the crate's
own rule the loop decides it. I wrote `self-duck.md`, read the README, and deleted it; iteration 46 relied on the same
fact for its tenth sentence.

## The design

1. **An eleventh sentence** in `problems()`: *"the duck on `Effects` starts on the bus it lowers, so every voice on it
   quietens itself"*. Written as the **mechanism**, not a verdict — a self-sidechain is a real device, and a project
   that meant one reads a true sentence and ignores it.
2. **The doc gains an eleventh bullet**; "Eleven checks".
3. **`problems_names_every_mistake` gains the sentence**, with a dedicated `Duck::new("Music","Music").hold_s(0.25)` row
   (it must carry a hold or the *tenth* sentence fires from it too, and the assertion passes for the wrong reason — the
   mistake iteration 46 made).
4. **`SIREN_MIX=broken`'s ducks are chosen so the block's three-sentence cap shows both new sentences**, and the
   sentence was **shortened from 118 to 78 characters** after the capture showed it wrapping onto a second line.

## Fields

None. `source` and `target` already exist; this reports their coincidence.

## Tests

- **`tests/mix.rs::a_duck_whose_source_is_its_target_lowers_the_bus_that_fired_it`** — the measured behaviour: `0.0`
  with no voice, `-6.0` on the frame one starts, and `bus_db` that much lower.
- **`problems_names_every_mistake`** — the sentence, attributable, mutation-checked.
- **Two guards added while probing and kept** because they pin behaviour nothing had asserted:
  `a_looping_cue_at_one_instance_never_accumulates` (60 requests, one voice) — correct, no bug found.

## The carve

- **Not** changing the behaviour: a self-duck still works. A project using one as a compressor loses nothing.
- **Not** a refusal: the crate draws the line at *table* faults → `problems()`, and a refusal here would silence a whole
  bus on one mistyped field.
- **Not** the `Duck::new` zero-hold default (backlog #2): that is a behaviour change to every bare rule and does need a
  decision file.
