# carve — a budget experiment that was reverted, and why

`cues.yaml` case 8 asks for *"Two voices live, one steal logged, and no third `Played`"* from three requests in one frame,
with `bus_max_voices: 2` and **no `max_instances`** on the cue — so the cue keeps its default `max_instances: 1`.

Measured, the crate produces **3 `Played`, 0 refused, 1 live voice**: each request takes the one previous voice of its own
cue (the cue cap), so the bus never exceeds 2 and the log shows two steals. That is what `max_instances: 1` means and the
crate is self-consistent.

I tried a fix — an unlimited cue (`max_instances: 0`) must not eat its bus, so `room_for` would refuse at the bus cap —
and the crate's **existing** `tests/mix.rs::a_full_bus_steals_the_oldest_and_then_refuses` showed it breaks a documented
behaviour (a cue at `max_instances: 0` *does* steal to stay within its bus). **Reverted**; `src/play.rs` is untouched in
this iteration.

**What should change** is the case's own `authored` block (`max_instances: 2`), which would make its `green_when` true of
the crate. That is a `cues.yaml` edit, and the loop's rules say the file grows only for a new behaviour, so it is
recorded here for the owner rather than made.
