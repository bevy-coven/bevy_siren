# plan.md — iteration 44: a layer waiting for a voice says so

## The pick, and why it beats the runner-up

**Pick:** `MusicState::blocked(layer)` — the **virtual voice** state — and the panel line that shows it. A score may name
more layers than the budget has room for; when a layer's `Play` is refused the plugin records the live count and retries
when it falls. That state was **private**, so the `Music` block drew `-80.0 dB` for a *waiting* layer and `-80.0 dB` for
one authored silent: one string, two answers to the question a panel exists for.

Beats the runner-up (`cues.yaml` case 32 as the item): writing the case is bookkeeping for iteration 43's work, and this
iteration did that too — but the case is worth nothing until the *state it describes* is observable, and the waiting
layer was not.

## The blocked items, re-checked

- **Case 31** (a non-finite cue `gain_db`) — still blocked on `needs-decision/bad-gain-problem.md`. Its own
  recommendation is *"C now, A at the next minor"*; the crate is at 0.1.0, so A waits for the release that moves it.
- Nothing was built from a blocked decision this iteration.

## The design

1. **`MusicState::blocked(&self, layer: &str) -> Option<usize>`** — the live count at which the layer's start was refused,
   or `None`. An **additive method** on an existing public type: no variant, no field, no exhaustive `match`.
2. The panel appends `waiting` to a layer's gain when `blocked` is `Some`.
3. **`SIREN_TIGHT=1`** gives the `Music` bus **one** voice, so the state occurs in a capture. It caps the bus, not the
   table: the first draft set `max_voices: 1` on the whole table, and the capture then showed six `refused` lines for
   every other play in the script — the waiting layer was proved, and the rest of the panel was starved.

## Fields

**No new field.** `blocked_at` already existed on the private `Layer`; only the reading was missing.

## Modules touched

- `src/music.rs` — the accessor.
- `examples/mixer.rs` — the `waiting` marker, `SIREN_TIGHT`.
- `tests/music.rs` — the existing blocked test now asserts the accessor, before and after a voice frees.
- `docs/loop/cues.yaml` — **cases 32 and 33**.
- `docs/guide/music.md`, `docs/media/README.md`, `docs/media/mixer-waiting.png`.

## Tests

- `tests/music.rs::a_refused_layer_start_is_tried_again_when_room_frees` — now asserts `blocked` is `Some(1)` for both
  layers while the budget is full, `None` for a layer the score lacks, and `(None, Some(1))` after one slot frees: the
  layer that got a voice stopped waiting, the other still is.
- **Mutation:** `blocked` returning `None` reddens it (verified). The retry is gated on the same number, so the same
  mutation also breaks the promotion.

## The carve

- **Not** a `MusicScore` or `MusicState` field: the state existed, and a second copy would be a second truth.
- **Not** a `Refusal` variant for a blocked layer: `Refusal::VoiceLimit` already answers, once per refusal, and the
  suppression while the count is unchanged is deliberate.
- **Not** the whole-table budget for `SIREN_TIGHT`: it starves the demonstration of everything else.
