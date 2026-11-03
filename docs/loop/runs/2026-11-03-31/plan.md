# plan.md — iteration 31: the bus row names the cue that ducked it

## The pick, and why it beats the runner-up

**Pick:** the bus row shows `duck -8.0 dB` and never says **by what**, so a person must find the matching log line to learn
which cue lowered the bus — while `Entry::Ducked` records it and `SirenMix::deepest_duck`'s own doc says it is *"for a panel
that says what a line of dialogue did"*. A method written for this panel and unused by it.

Beats the runner-up (the header's counts): the header already counts cues, buses and voices, and a duck count would repeat
what the bus rows show. This one is a **missing fact on the row a person is reading**.

## The design

In `examples/mixer.rs`'s `Kind::Bus` arm, while the bus is ducked, annotate with the **newest** `Entry::Ducked` naming that
bus from the `SirenLog` the panel already holds:

```
Music       -9.5 dB   duck -8.0 dB by dialogue.greeting   2 / 12 voices
```

- **No new public field:** `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel holds both.
- **No staleness:** the clause renders only inside `duck < -0.05`, so it vanishes the frame the duck does.
- **Fits:** 71 characters against ~114 columns at 1280×720; the capture is the proof.

## Fields

None. No `src/` change (only `examples/mixer.rs`), no public type, no schema.

## Tests

- `tests/mix.rs::the_newest_duck_naming_a_bus_is_the_one_a_panel_shows` — the **rule** the panel uses, pinned where the suite
  can see it: a non-duck entry between two ducks does not confuse it, a bus nothing ducked has no name, and a newer duck on
  the same bus wins. It is a guard, not a proof of a fixed bug: the view is newer than the rule.
- Six captures looked at: 1280 with a duck, 1920 with a duck, a later frame, and — through a probe — the **released** state.
- Full gate; counts rise by one on each feature set (113 / 120 / 122 + 5).

## The carve

- **Not** a `deepest_duck` line in the header: the rows already carry the depth per bus, and a header that repeats the rows
  is noise. `deepest_duck` remains available to a host that wants the single deepest.
- **Not** a new `SirenMix` field for the ducking cue: a public field is an API change and the log already answers it.
