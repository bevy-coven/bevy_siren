# research.md — iteration 55: a cue the table dropped stops holding a picker

## The item

`SirenChoices` is the crate's one random stream plus **a `VariantPicker` per cue name**:

```rust
pub struct SirenChoices {
    pub rng: Rng,
    seeded: u64,
    pickers: HashMap<String, VariantPicker>,   // created on a cue's first play, removed by nothing
}
```

`answer` → `choices.pick(cue)` inserts an entry keyed by `cue.name`. `reset_histories` walks the map and clears each
picker's **contents**; it never removes an entry. Nothing else touches the map.

**Measured**, driving 200 cue names through the real play path, then cutting the table to ten:

| | pickers held |
|---|---|
| after 200 plays | 200 |
| after `reset_histories` | **200** — it is not a retain |
| after the table is cut to 10, one frame | **200** — before the fix |

## Why this is rank 1

Iteration 54 fixed the identical shape in `DuckClock::started`. This is the same fault one resource over: a map keyed by
a name the table owns, never reconciled with the table. A project that **generates cue names** — one per level, per enemy,
per dialogue line, the pattern `docs/research/siren.md` records Franco on (*"the game emits a play request that says what
kind of sound it wants"*) — grows one picker per name it has ever played, and a `VariantPicker` holds a `VecDeque<usize>`
and a **`VecDeque<String>`** with a copy of every recent sound key.

That last part is the cost that matters: not just a few bytes of map entry, but a duplicated string per remembered sound.

## Sources

**S1. `Simon Franco, Creating Dynamic Soundscapes Using an Artificial Sound Designer, Game AI Pro ch. 46`** — *"the game
emits a play request that says what kind of sound it wants and which object it belongs to, and the audio system decides
which sample to play"* (`docs/research/siren.md`). The **request carries the kind**; the name is a runtime value, not a
fixed table entry, which is exactly how a generated-name project arises.

**S2. `Gregory, Game Engine Architecture 3e §14.5.8`** — a **virtual voice** is *"a sound the game believes is playing but
that has no hardware voice"*; a mixer enforces per-group instance limits. The chapter's theme is that a sound system
**budgets** what it holds. A picker map bounded by nothing is the same discipline applied to one resource and not the
next.

**S3. `VariantPicker`'s own doc** — *"A picker that remembers the last `capacity` picks"*; `remember` stores
`key.to_owned()` per pick (`src/model.rs:445`). So each live picker owns a copy of its recent sound names. Dropping a
picker is not a bookkeeping tidy-up; it frees strings.

**S4. `CueTable::get` is a linear scan** (`src/cue.rs:363`: `self.cues.iter().find(...)`). That fact decides the
implementation, and is why the obvious retain was measured rather than assumed — see `perf.md`.

**S5. The crate's own precedent** — iteration 54's `DuckClock::retain_rules`, the same split: *"is this envelope over?"*
the clock can answer alone; *"is this still a rule?"* only the table can.

## The technique

`SirenChoices::retain_cues(&table)` drops every picker whose cue the table no longer has, and `play_requests` calls it —
**only on a frame where the table changed**, because a static table makes the same answer every frame.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| pickers held after 200 plays, table cut to 10 | **200**, before | measured |
| `reset_histories`' effect on the map | **none** | measured, S3 |
| naive retain, 1000 cues × 1000 pickers | **990 µs/frame** — 6 % of a 60 Hz frame | measured |
| one-pass retain, 1000 cues | **8.5 µs/frame** | measured |
| one-pass retain, 5000 cues | **34–55 µs/frame** | measured |
| with the change-detection guard, unchanged table | **0 ns** | measured |
| a remembered sound key's storage | one owned `String` per pick, per picker | S3 |

## What good sounds like

A picked stream that is byte-identical to before — the fix changes no variant choice — and a map whose size follows the
table: 200 names played, table cut to ten, ten pickers on the next frame the table is written, and none of the freed
pickers' sound keys held. `SirenChoices::picker_count()` is the number, so a panel can draw it and a test can assert it.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`**,
the fifth iteration running. The sources here are the vendored `docs/research/siren.md` plus measurements against this
crate.
