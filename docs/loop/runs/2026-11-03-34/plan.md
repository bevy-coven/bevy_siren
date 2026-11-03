# plan.md — iteration 34: a layer is a stem of the Music bus

## The pick, and why it beats the runner-up

**Pick:** `MusicLayer::cue`'s doc says the cue *"should name the `Music` bus"*, and **nothing checked**. A layer naming a loop
on `Effects` **started there**: `MusicState` reported two layers live while one was mixed on the effects bus, `MusicLayers`
could not move it, and `MusicScore::problems` said **nothing**. The stinger path already refuses exactly this
(`play_stingers`); the layer path had neither the check nor the line.

Beats the runner-up (the panel's Music block, deferred twice for height): this is the **score and the sound disagreeing**,
rank 1, and it is silent.

## The design

1. **`start_missing` refuses a layer whose cue's bus is not `Music`**, through the machinery iteration 16 built for a
   non-looping layer: one notice per layer, and the layer is not started. The `bad` list carries `(name, cue, sentence)`
   so both faults share one loop and **the reason is the thing carried**, not re-derived at the print.
2. **`MusicScore::problems` gains the sentence**, word for word, so the report and the log cannot disagree — the same
   discipline iteration 19 used for a gain that is not a number.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `bad` is private; no public type changed; the score's report is
`Vec<String>`, so a sentence is not an enum change.

## Tests

- `a_layer_on_the_wrong_bus_is_not_started_and_is_reported` — reported once with the layer and the bus; only the `Music`
  layer plays; `live() == 1`; one log line containing *"a layer is a stem"*. **Fails on the base commit** (`left: []`, and
  both layers played).
- `a_layer_on_the_music_bus_still_starts` — the guard: a good score starts both layers and reports nothing. A rule that
  refused every layer would pass the first test alone.

## The carve

- **Not** a `Refusal` variant for the layer case: the score's report is `Vec<String>` and the log carries the sentence;
  `Refusal` is the play path's vocabulary, and `needs-decision/stinger-refusal.md` already records the same question for
  the stinger.
- **Not** the panel's Music block, again deliberately: the height budget is measured (12 px free, ~134 needed) and the
  panel is rank 4.
