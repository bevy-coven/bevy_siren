# research.md — iteration 10: the mix forgets a bus that left the table

## The item

`update_mix` writes `SirenMix::db` and `SirenMix::ducked` **by insertion and never prunes**. A project that swaps
`SirenSettings` — a cutscene mix with two buses, a menu with one, a boss fight that adds a bus — leaves every bus from
the previous table in the public maps, at its last level:

```
four default buses:      db = ["Ambience", "Dialogue", "Effects", "Music"]
after swapping to two:   db = ["Ambience", "Dialogue", "Effects", "Music"]   ← unchanged
                         bus_db("Ambience") = -3.3 dB   for a bus not in the table
                         deepest_duck()     = Some(("Ambience", -12.0))   ← a phantom, forever
```

Three things are wrong and they compound:

1. **`bus_db(name)` answers for a bus that is not in the table.** It is documented *"The gain `bus` is playing at this
   frame, or [`SILENT_DB`] for a bus nothing has started yet"* — a bus that has **left** the mix is not "not started
   yet", and returning a level is worse than returning silence.
2. **`deepest_duck()` names a ghost.** It iterates `ducked` and takes the minimum, so a bus that has left the table —
   still carrying its last duck — is reported as *the* bus a player is hearing ducked. That is the panel's "why is the
   music quiet" answer pointing at a bus that no longer exists.
3. **A duck on a removed bus never recovers in the map.** `duck_db` is evaluated per bus in `settings.buses`, so a
   removed bus's last value is frozen at its deepest.

Nothing crashes, no test fails, and the audible mix is correct — the stale entries are written nowhere. This is a
**readable-state** bug: the crate's own rule R6 says *"`SirenMix` is one number per bus, so a panel can draw it"*, and
the number is for a bus the table does not have.

## Sources

1. **The crate's rule R6** (`docs/research/siren.md`): *"[`SirenMix`] is this frame's mix — one number per bus, ducks and
   master included — for a panel or a game's own meter."* **This frame's.** A value left over from a previous table is
   not this frame's mix; it is a cache with no owner.

2. **`SirenMix`'s own docs**: `db` is *"each bus's gain this frame"*, `ducked` is *"each bus's current duck in dB"*. A
   bus that is no longer in `SirenSettings::buses` has no gain and no duck this frame, and both fields say "this frame".

3. **Bevy 0.20's resource model** (`bevy_ecs`): `insert_resource` replaces a resource wholesale, so a project changing
   its mix table is the ordinary path — not an edge case. `SirenSettings` is documented as a file *and* a resource a
   game can insert.

4. **The panel's reach** (`bevy_cauldron::audio_panel::bus_rows`) iterates the buses it was given, so the ghost rows do
   not appear there; but `deepest_duck()` and any consumer iterating `mix.db` — a game's own meter, a test, an agent
   reading the world — see the phantoms. The panel itself *calls* `bus_rows` with the table's buses, so it is safe by
   construction; the **crate's own public accessors are not**.

5. **`DuckClock`'s `prune`** already exists and prunes **manual** envelopes that finished recovering. The mix maps are
   the one piece of per-bus state with no equivalent, which is the inconsistency this iteration closes.

**Literature server unavailable, eighth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is argued from the crate's own R6 and its field docs, which is where a "this frame"
contract belongs.

## The technique

**The maps are rebuilt each frame, not updated in place.** Two options, and the cheap one is right:

- **`retain` before inserting**: `mix.db.retain(|name, _| names.contains(name))` and the same for `ducked`, once per
  frame before the bus loop. `BTreeMap::retain` is O(n) over the current entries, n = the table's buses plus whatever
  is being dropped, and it runs where the code already iterates the buses. `voices` is already rebuilt wholesale
  (`mix.voices = counts`), which is why *it* has never had this bug — the two maps beside it are the exception, not the
  rule.
- **Rebuild**: `mix.db = targets.iter().cloned().collect()`. Same result, one allocation per frame, and it would drop
  the "a bus nobody has written yet starts at its target" behaviour, which depends on reading the previous value. So
  `retain` it is.

One predicate decides it: the buses the table has **this frame**. Anything else is not in this frame's mix.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what `db` and `ducked` contain | one entry per bus in `SirenSettings::buses`, this frame | R6; both fields' docs |
| what `bus_db(unknown)` returns | `SILENT_DB` | its doc, unchanged — a bus that left is now "unknown" |
| when the prune runs | once per frame, before the bus loop | design default: same place `voices` is rebuilt |
| cost | one `retain` over the previous map, n ≤ the table's buses | `BTreeMap::retain`, O(n) |

No new field, no new number, no public type change.

## What good sounds like

Nothing audible changes — the stale entries were never written to a sink. What changes is what a **reader** sees:

| after a bus leaves the table | before | after |
|---|---|---|
| `mix.db.keys()` | still lists it | gone |
| `mix.bus_db("Ambience")` | `−3.3 dB` (a level for a bus that is not there) | `−80.0 dB` (silence, the documented answer for a bus the mix does not have) |
| `mix.deepest_duck()` | `Some(("Ambience", −12.0))` forever | `None`, or the bus that is actually ducked |

A reviewer judges it without ears: the probe's three lines, before and after, plus `cues.yaml` case 29. The
**non-regression** is that a bus which is *in* the table keeps its smoothed approach — the `current` read that makes a
fader move instead of step must still find its previous value, which a rebuild would destroy and `retain` does not.
