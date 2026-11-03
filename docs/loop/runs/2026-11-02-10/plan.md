# plan.md — iteration 10: the mix forgets a bus that left the table

## The pick, and why it beats the runner-up

**Pick:** `SirenMix::db` and `SirenMix::ducked` are written by insertion and never pruned, so after a project swaps
`SirenSettings` the public maps still hold every bus from the old table — at its last level, with its last duck. Probed:
`bus_db("Ambience") = −3.3 dB` and `deepest_duck() = Some(("Ambience", −12.0))` for a bus the table no longer has.

**Runner-up:** the five decision files. All are public-enum questions the owner has not answered; none is buildable.

The pick wins on the loop's first criterion as a **readable-state** bug: the crate's rule R6 is that `SirenMix` is one
number per bus so a panel can draw it, and it draws a bus that does not exist. `deepest_duck()` is the crate's own
answer to "why is the music quiet", and it answers with a phantom.

**Carved off:** pruning `DuckClock.started` of rules whose source/target left the table. That is a second map with the
same shape of staleness, but its keys are `(source, target)` pairs and its values are start instants; a stale pair is
inert once no bus matches. `manual` is already pruned. Named in the log's next-5, not fixed here.

## Design

`src/mix.rs` only.

```rust
// **This frame's mix has this frame's buses.** A project may swap `SirenSettings` at any time — a cutscene mix with
// two buses, a menu with one — and a value left from the previous table is not this frame's mix. `voices` is rebuilt
// wholesale below; these two maps were the exception.
let buses = settings.bus_names();          // already a method on SirenSettings
mix.db.retain(|name, _| buses.contains(&name.as_str()));
mix.ducked.retain(|name, _| buses.contains(&name.as_str()));
```

placed **before** the bus loop, so the loop still reads the previous value of a bus that is still in the table — that
read is what makes a fader approach instead of step.

`settings.bus_names()` returns `DEFAULT_BUSES` for a table that lists none, so an empty `buses: []` resolves to the
crate's four exactly as `answer` already treats it — the prune and the play path agree by construction.

## Every field touched on a serde type

None. The prune reads `SirenSettings::buses` through `bus_names()`, which exists.

## Tests that will prove it

`tests/mix.rs`:

1. `a_bus_that_left_the_table_leaves_the_mix` — the probe as a test: default settings for two frames (Ambience gets a
   duck), then a table of `Music` and `Effects`; after a few frames `mix.db` has exactly those two keys, `bus_db("Ambience")`
   is `SILENT_DB`, and `deepest_duck()` is `None`. **Fails on the unfixed tree** on all three assertions.
2. `a_bus_that_stays_keeps_its_smooth_approach` — the non-regression a rebuild would break: a bus in the table on both
   frames moves toward its new level rather than stepping to it, because the prune left its previous value readable.
3. `an_empty_bus_list_still_means_the_crates_four` — `buses: vec![]` keeps the four default rows in `mix.db`; the prune
   must agree with `SirenSettings::bus_names()` and with `answer`'s `UnknownBus` check.

`tests/table.rs` is untouched: no serde field moves.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 6, the dialogue duck, reads `mix.ducked["Music"]` and must
  stay green);
- `cargo run --release --example perfcheck` before/after — two `retain`s per frame over a map whose size is the bus
  count (four by default), so the cost is a handful of comparisons; measured, not assumed;
- `examples/render` before/after: cue WAVs byte-identical, since no level that is written changes.

## Risk

`retain` drops a value the bus loop would otherwise read as `current` for a **returning** bus: a project that removes
`Ambience` for one frame and puts it back gets a bus that starts at its target rather than easing from its old level.
That is the same behaviour as a bus nothing has started yet ("starts at its target rather than fading up from
nothing"), which is documented and is what a table swap should do.
