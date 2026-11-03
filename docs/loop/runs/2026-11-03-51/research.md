# research.md — iteration 51: a picker's window is what the cue asked for, not what its first play sized

## The item

`SirenChoices::pick` builds each cue's [`VariantPicker`] on **its first play**:

```rust
let capacity = usize::from(avoid).max(1);
let picker = self.pickers.entry(cue.name.clone()).or_insert_with(|| VariantPicker::new(capacity));
picker.pick_from(&keys, avoid, &mut self.rng)      // <- `avoid`, read fresh from the cue
```

`pick_from` takes `avoid` **per call**, but the window it can refuse is capped by the picker's `capacity`, fixed at
construction. So a cue whose `avoid` grows after its first play silently keeps the old window.

**Measured** (`VariantPicker`, four keys, seed 11, 24 picks):

| picker | `avoid` asked | sequence | longest run |
|---|---|---|---|
| `new(1)` | 1 | `adbcbcabcacabcabcbabcbcb` | 1 |
| `new(1)` | **3** | `adbcbcabcacabcabcbabcbcb` | 1 — **identical to `avoid: 1`** |
| `new(3)` | 3 | `adbcadbcadbcadbcadbcadbc` | 1 — cycles cleanly |

The middle row is the defect: the author asked to avoid three variants and got one, and nothing says so.

**Reachable.** `reset_histories` exists but is called **nowhere** in this crate, its tests, or the Cauldron — so the
picker outlives every table reload. The Cauldron's audio panel rewrites `family/bevy_siren.ron` and reads it back
(`audio_panel.rs:118`); a cue whose `avoid` is raised there keeps its first play's capacity for the life of the app.

## Sources

**S1. `Repeat::AvoidRepeat`'s own doc** — *"Random, but never one of the last `avoid` variants — the default, and what
kills the machine-gun footstep."* The guarantee, in the crate's words.

**S2. `VariantPicker::new(capacity)` — *"A picker that remembers the last `capacity` picks."* The window is the picker's,
so a `capacity` that disagrees with `avoid` is a contradiction, not a preference.

**S3. `pick_from`'s own comment, line 382** — *"A cue of one sound repeated has no alternative at all, and refusing it
would be the crate inventing a refusal for a table that is simply thin. **It plays, and `problems()` is where the table
should be named.**"* This promise is **also unkept**: `problems_with` never looks at `avoid`, and a probe on a two-variant
cue with `avoid: 5` returns `problems: []`.

**S4. Böttcher, on the problem R1 solves** — *"the main problem of procedural audio is not the synthesis but the
management of thousands of assets"*, met with *"alternative recordings and randomisation"* (`docs/research/siren.md`
records the quote; home-still was unavailable — see *Blocked*). The variation table is the crate's answer to asset
repetition, and a window that silently collapses to one is the machine-gun footstep returning for a different reason.

**S5. Gregory, Game Engine Architecture 3e §14.6** — variation is per-play state, not per-table state; the picker is
history, so its size is a runtime decision.

## The technique

Make the picker's memory **follow the rule it is asked to enforce**, not the rule that happened to be in force when the
cue was first played. Two changes, both internal:

1. **`VariantPicker::set_capacity(n)`** — grows or shrinks the window before a pick, so `pick_from`'s `avoid` is always
   the number honoured. Public because `VariantPicker` is already public and a host building its own picker needs it;
   additive, so no break.
2. **`SirenChoices::pick`** calls it before `pick_from`, so a table reload that raises `avoid` takes effect on the next
   play.

**`reset_histories` gains a caller**: the Cauldron's table reload should reset. That is a consumer change in
`bevy_grimoire`, its own commit.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| window a picker can refuse | its `capacity`, **not** the `avoid` passed to a pick | S2 — measured |
| `avoid: 3` on a `capacity: 1` picker | identical to `avoid: 1` over 24 picks | measured, this iteration |
| `avoid: 3` on a `capacity: 3` picker | cycles with longest run 1 | measured |
| default `avoid` | 1 | `Repeat::default()` |
| `SirenChoices::pick`'s capacity | `avoid.max(1)` **at first play only** | `src/play.rs:290` |

## What good sounds like

A four-variant footstep at `avoid: 3` cycles **`adbcadbc`** — no key within three of itself — instead of the drifting
`adbcbcabcacabcab` a one-deep window gives. On a spectrogram the difference is a repeated recording every third step
versus a genuine rotation; on the panel it is a log where the same sound does not reappear until three others have.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`** on
both queries tried. The literature quoted here is the vendored `docs/research/siren.md`, which already records
Böttcher's and Gregory's numbers for R1; no new DOI was needed for a defect that is proved by the crate's own arithmetic.
