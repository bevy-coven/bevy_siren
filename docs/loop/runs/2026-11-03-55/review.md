# review.md — iteration 55, three passes

## Pass 1 — Function: does a dropped cue stop holding a picker?

**The claim** (`docs/research/siren.md`, Franco): a play request *"says what kind of sound it wants and which object it
belongs to"* — the name is a runtime value, so a project that generates names is the ordinary case, not an exotic one.

**Measured**, 200 names through the real play path, then the table cut to ten:

| | pickers held |
|---|---|
| after 200 plays | 200 |
| after `reset_histories` | **200** — clears contents, is not a retain |
| after the table is cut to 10 | **200 before, 10 after** |

`evidence/plots/pickers-held.svg` draws the two paths: red flat at 200 for ever, blue dropping to 10 on the frame the
table changes and staying there.

**The test runs on both trees.** `the_picker_map_grows_per_cue_name` reads the map through `Debug` — no new API — so it
compiles against the old crate and fails there at
`assertion left == right failed: a cue the table dropped must not keep its picker — left: 200, right: 10`.

**Verdict: GREAT.**

## Pass 2 — Zoom: the cost, and a regression I put in and took out

**The obvious fix was 116× too slow, and measuring found it.** `CueTable::get` is `self.cues.iter().find(...)` — a linear
scan — so `pickers.retain(|name, _| table.get(name).is_some())` is O(pickers × cues) **every frame**:

| cues × pickers | naive retain |
|---|---|
| 10 × 10 | 1.5 µs |
| 100 × 100 | 20.6 µs |
| 1000 × 1000 | **989.8 µs** — 6 % of a 60 Hz frame |

**One pass over the table** into a `HashSet<&str>` is O(cues + pickers): **8.5 µs** at 1000 cues, 34–55 µs at 5000.

**Then the guard**, because a static table makes the same answer every frame and 8.5 µs a frame for a session-long
constant is the same fault as 990: `play_requests` retains only when `table.is_changed()`, and `retain_cues` returns
immediately on an empty map. **0 ns** for both, measured.

**The edge case change-detection could break, tested.** A write in one frame and plays in later ones: the fix must not
need the play to trigger the retain. `a_shrunken_table_still_drops_pickers_after_later_frames` asserts 50 → 5 on the
frame the table changes, still 5 two frames later, and still 5 after playing a surviving cue.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: did any choice move?

**The streams are byte-identical to iteration 54's:**

```
variants footstep-grass: avoid 1, 3 sounds -> step_03 step_01 step_02 step_03 step_02 step_03 step_01 step_02 step_03 step_02 step_01 step_03
variants footstep-stone: avoid 4, 2 sounds -> stone_02 stone_01 stone_02 stone_01 ...
```

A retain that dropped a **live** cue's picker would reset its history and change the next variant — a worse bug than the
leak, and exactly the kind a "cleanup" introduces. It did not: the retain keys on the table's cue names, and a cue the
table keeps keeps its picker object, history and window intact.

**Audio unchanged**, as a report-and-memory fix should be: `bus-Music` reads `duck -8.0 dB, deepest -7.96 dB, recovered
by 1.97 s`; `mix.wav` integrates **−18.6 LUFS at −6.1 dBFS**.

**Can a developer read it without the code?** Yes — `picker_count()` is the number, the plot is the story, and the guide
says which method is which: `reset_histories` clears histories, `retain_cues` follows the table.

**Verdict: GREAT.**
