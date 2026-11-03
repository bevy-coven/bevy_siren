# plan.md — iteration 42: the cue row says how many may sound, and a steal says which cap

## The pick, and why it beats the runner-up

**Pick:** the cue list has never shown `max_instances`, the one field that separates "two footsteps may overlap" from
"one click at a time" — and the log said `stolen by <cue>` for **three different caps**, so an author could not tell
which number to raise.

Beats the runner-up (a live-layer count in the `Music` header): that is decoration on a block that already names each
layer's gain; this is the panel failing to show a **budget that refuses plays**, and the crate's own vocabulary (a
`CueProblem` for `max_instances: 0`, a `Refusal` that names its bus) already says the number matters.

## The blocked item, and why it was not built

Backlog #1 (`Stopped` on a parent's despawn) is blocked on `needs-decision/remove-completes.md`. `cues.yaml` **case 31**
(a non-finite `gain_db` reported by `problems()`) is red and blocked on `needs-decision/bad-gain-problem.md`: every route
to it — a `CueProblem` variant, a `Refusal` variant, a new `Entry` variant — touches a **public enum with no
`#[non_exhaustive]`**, which is a breaking change the file says a patch release should not carry. Checked this iteration:
`grep non_exhaustive src/` finds none. So case 31 stays red and the decision stays open; nothing to build.

## The design

1. **`{poly:>3}`** in the cue row: `x{n}`, or `all` for the documented `max_instances: 0` (which `problems()` already
   reports as `UnlimitedInstances`, so the row does not invent a number).
2. **`Step::Poly`** — three `footstep.grass` requests in one frame, last in the default script.
3. **`room_for` returns `Vec<(Entity, Cap)>`**, and `Cap::as_str()` gives `"cue"`/`"bus"`/`"table"`, so the stop sentence
   is `stolen by footstep.grass (cue)`.

## Fields

**No new field** on `CueTable`, `SirenSettings` or `MusicScore`. `Cap` is private, and `Started.stolen` is private.

## Modules touched

- `examples/mixer.rs` — the column, the step, the script order.
- `src/play.rs` — `room_for`'s return type, `Started.stolen`, `Cap::as_str`, the sentence.
- `src/log.rs`, `src/mix.rs` — two doc comments that name the sentence's form.
- `tests/mix.rs` — `a_steal_names_the_cap_that_took_the_voice`, and a `(cue)` assertion added to
  `a_cue_respects_its_own_polyphony`.

## Tests

- `tests/mix.rs::a_steal_names_the_cap_that_took_the_voice` — forces the **bus** cap with `max_instances: 4` and a
  one-voice bus, asserts the sentence ends `(bus)`.
- `a_cue_respects_its_own_polyphony` — now also asserts the cue cap's sentence ends `(cue)`.
- **Mutation:** replacing the format with `format!("stolen by {}", request.cue)` reddens **both** (verified). Every
  existing `starts_with("stolen by")` assertion still passes, which the run confirms.

## The carve

- **Not** a `Cap` in `Refusal::VoiceLimit` — iteration 24 already did that, and `bus: Option<String>` covers it.
- **Not** a `problems()` check for a cue whose `max_instances` exceeds its bus's cap: that is not a fault (the bus
  steals), and reporting it would teach an author to ignore the report.
- **Not** a new `Entry` variant — a public enum, so a decision file, not an iteration.
