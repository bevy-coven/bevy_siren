# research.md — iteration 54: a duck rule the table dropped stops holding its start

## The item

`DuckClock` holds two collections:

```rust
pub struct DuckClock {
    started: BTreeMap<(String, String), f64>,   // one entry per (source, target) that ever fired
    manual: Vec<ManualDuck>,                    // envelopes nobody's cue started
}
```

`prune(now_s)` retained only `manual`. Nothing ever removed a `started` entry except `clear()`, which nothing calls.

**Measured** (`DuckClock` derives `Debug`, so the map is observable without an accessor):

| scenario | before |
|---|---|
| 2000 `fire`s, then `prune` at t=1000 | **2000 entries still held** |
| fire `Dialogue->Music` and `Voice->Music`, then `prune` at t=1000 | `("Dialogue", "Music")` **still held** |
| 100 table edits, each firing its own rule, then `prune` at t=1000 | **100 entries** |

That is an unbounded map keyed by two strings per entry, in a resource that lives for the process.

## Reachability

**`DuckClock::fire` and `manual` are public** — a host may push a duck for any pair it likes, and nothing removes it.

**The crate's own path is narrower but still leaks.** `play_requests` calls `clock.fire_all(&settings, &started.bus, now)`,
which fires only rules in the **current** table (`src/play.rs:438`). So the leak grows when the table's set of rule pairs
*shrinks or changes* — a renamed bus, a deleted duck, a reload. The Cauldron's audio panel rewrites and re-reads
`family/bevy_siren.ron` (`audio_panel.rs:118`), so a designer editing a duck rule a hundred times during a session leaves
a hundred entries. Small per entry, permanent per edit, invisible in every view the crate exposes.

## Sources

**S1. `Gregory, Game Engine Architecture 3e §14.6.3` — ducking is a group lowering another group for a while.** *"For a
while"* is the whole of it: a duck is an **envelope with an end**, which is why the crate keeps a start time rather than
a volume. An envelope that outlives its rule is not a longer duck; it is a record of a duck that no longer exists.

**S2. The same chapter on voice limits** — *"the number of simultaneous sounds in a group is limited, and when the limit
is reached…"* (`docs/research/siren.md:106`). The crate already spends its budget carefully (`max_voices`, `steal`,
`SirenMix::worst_peak_db`); a map that grows without limit beside that budget is the same discipline applied to one
resource and not the next.

**S3. `DuckClock::prune`'s own doc** — *"Drops manual envelopes that have finished recovering: they are over, and
keeping them would grow the list for the life of the process."* The sentence names exactly the fault, for one of the two
collections. The other had no such rule.

**S4. `SirenMix`'s bus pruning precedent** (iteration ~46, `tests/mix.rs:498`) — *"a project that swapped `SirenSettings`
— a cutscene mix with two buses, a menu with one — kept every bus"*, fixed by retaining only current buses. The same
shape, one struct over.

## The technique

Split the two questions that `prune` was conflating:

- **Is this manual envelope finished?** — answerable from the clock alone, `now_s` and the envelope. Stays in `prune`.
- **Is this rule start still a rule?** — answerable only from the table. Becomes `retain_rules(&settings)`, called by
  `update_mix` beside `prune`.

`retain_rules` drops a `(source, target)` entry when no rule in `settings` names it. A manual duck is not a rule and is
untouched; it is `prune`'s.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a duck's lifetime | `hold_s + fade_s`, then the envelope is over | S1 |
| entries held after 2000 fires + prune | **2000**, before | measured |
| entries held after 100 table edits + prune | **100**, before | measured |
| entries after the same, with `retain_rules` | **1** — the rule the table still names | measured |
| a live rule's depth across a retain | **unchanged at −8.0 dB** | measured |

## What good sounds like

The same duck, and a clock whose size is bounded by the table rather than by the session: a rule the project edited
away stops holding a start on the next frame, and a rule still in the table keeps its envelope exactly — no restart, no
shortened fade. `DuckClock::rule_starts()` is the number, so a panel can draw it and a test can assert it.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`**,
the fourth iteration running. The numbers here are measured against this crate and cited to the vendored
`docs/research/siren.md`; no new DOI bears on a collection that is never emptied.
