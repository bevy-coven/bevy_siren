# plan.md — iteration 5: a refusal that names the cap that refused

## The pick, and why it beats the runner-up

**Pick:** `Refusal::VoiceLimit` names the bus whenever any of the three caps fails, so a full *table* budget or a
cue's own polyphony reports ``no voice free on bus `Effects` `` while `Effects` has room. Proved with a probe: both
cases print that sentence today.

**Runner-up:** `Completes` can never be true (`needs-decision/remove-completes.md`). That is dead API surface, not
wrong output, and it needs the owner's answer.

The pick wins on the loop's first criterion — a refusal is the most valuable bug because it is silent, and this one is
worse than silent: it is **confidently wrong**, and it points a person at a bus row they will find empty.

**Carved off:** a payload that names the cap (`Cue`/`Bus`/`Table`) by name. That changes a public enum's field set, so
it goes to `needs-decision/refusal-names-its-cap.md`; the sentence today ships correct-per-the-`Option`-doc without it.

## Design

`src/play.rs` only.

```rust
/// Which of the three hard caps could not be made room in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cap { Cue, Bus, Table }

fn room_for(live: &[Live], cue: &Cue, settings: &SirenSettings) -> Result<Vec<Entity>, Cap>
```

and in `answer`:

```rust
let stolen = match room_for(live, cue, settings) {
    Ok(stolen) => stolen,
    // The bus is the subject only when the bus's own cap refused; a full table or a cue at its own
    // `max_instances` leaves every bus with room, and `VoiceLimit::bus` is documented "when one did".
    Err(Cap::Bus) => return Err(Refusal::VoiceLimit { bus: Some(bus) }),
    Err(_) => return Err(Refusal::VoiceLimit { bus: None }),
};
```

Three `return None` become `return Err(Cap::Cue)`, `Err(Cap::Bus)`, `Err(Cap::Table)`; `Some(stop)` becomes `Ok(stop)`.

## Every field touched on a serde type

None. `Cap` is private and not serialised; no `CueTable`, `SirenSettings` or `MusicScore` field moves, and no
`Refusal` field is added.

## Tests that prove it

`tests/mix.rs`:

1. `a_full_table_budget_names_no_bus` — `max_voices: 1`, one bus-wide budget, an unstealable cue: the refusal is
   `VoiceLimit { bus: None }`, and its sentence is `"no voice free"`. **Fails on the unfixed tree** (it says `Some`).
2. `a_cue_at_its_own_polyphony_names_no_bus` — `max_instances: 1`, priority 200, table budget of 8: `bus: None`.
   **Fails on the unfixed tree.**
3. `a_full_bus_names_that_bus` — the one case where the bus *is* the subject: `BusLevel::new("Effects").max_voices(1)`
   with two stealable cues; `bus: Some("Effects")`. This must **not** regress, and pairs with the two above so a
   build that stopped naming the bus altogether is red.
4. `every_voice_limit_sentence_matches_its_payload` — a table over the three caps asserting the exact strings, so the
   panel's line and the payload cannot drift.

`examples/render` gains the three refusal sentences (it already writes `refusal-*.txt`), so the before/after evidence
is three files whose text differs where it should.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (cases 7–9 are the three caps and must stay green, now with their
  payloads asserted);
- `cargo run --release --example perfcheck` before/after — the change is a `Result` instead of an `Option` on a path
  that only runs when the budget is full, so it should be invisible; measured, not assumed;
- `examples/render` before/after: every cue WAV byte-identical, and the three refusal texts differing from the
  before tree exactly where the cap differs.

## Risk

A consumer that matched `VoiceLimit { bus: Some(_) }` to mean "the voice limit refused" now sees it only for the bus
case. The Cauldron does not match on it (it prints `Entry::Refused`'s sentence, which comes from `Display`), and this
crate's own tests match with `..`. That is the one behavioural change for a host, and it is the correction: anyone who
wants "did the limit refuse at all" matches `VoiceLimit { .. }`, which still works.
