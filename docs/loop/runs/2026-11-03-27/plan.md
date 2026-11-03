# plan.md — iteration 27: a duck ends its hold when its voice does

## The pick, and why it beats the runner-up

**Pick:** a duck's hold is justified by a **voice** — `Duck`'s own doc says so — and `DuckClock` ran the full hold
whatever happened to the source. A line stopped on frame 0 left `Music` at −8 dB for another 15 frames and spent 0.6 s
recovering: **0.85 s of duck for a sound that had already stopped**. The skill names *"a duck that does not recover"* as
rank-1; this is that failure arriving late.

Beats the runner-up (a `SirenLog` `Recovered` entry, iteration 26's lead): that one is a **new variant of a public
enum**, blocked like the four `CueProblem`s, and the release is already readable from `SirenMix::ducked`. This one needs
no API change and fixes a sound a player hears.

## The design

1. **`DuckClock::source_went_quiet(&live, &settings, now)`** — for every rule whose `source` bus has no live voice,
   rewind `started` by the hold that had not yet elapsed. **Once only**: a rule whose hold already elapsed is left
   alone, or the fade would restart forever.
2. **`update_mix` counts live voices before the bus loop** (same predicate as `SirenMix::voices`: not `Stopping`) and
   calls it. The count loop that writes gains stays where it was.
3. **`duck_db_with(live, …)`** is the settings-and-voices-aware read; `duck_db` delegates with an empty map, so a caller
   not tracking voices keeps the old behaviour exactly.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `DuckClock` gains a method; `duck_db_with` is new and `duck_db`
keeps its signature.

## Tests

- `a_duck_ends_its_hold_when_its_source_is_stopped` — ten frames after a `StopCue`, the duck is recovering
  (`> -7.5 dB`) and not yet zero. **Fails on the base commit** (still `-8.0`).
- `a_duck_whose_source_keeps_playing_holds` — guard: a looping line at 0.20 s is still full `-8.0`.
- `a_second_line_extends_the_duck` — guard: a second play at 0.20 s restarts the hold, so at 0.40 s it is still full.

## The carve

- **Not** a `Recovered` log entry: public enum, decision file written this iteration
  (`needs-decision/duck-recovers-entry.md`).
- **Not** a change to `active_duck_db`: it is a pure function of elapsed time and stays one; the clock is what learns.
- **Not** ending a **manual** duck (a stinger's) when the stinger stops: a stinger's duck is authored for its own
  `duck_s` and is not tied to the voice, which is the documented distinction.
