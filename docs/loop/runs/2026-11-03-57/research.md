# research.md — iteration 57: a stinger that cannot dip does not claim it did

## The item

`arm_stinger_ducks` armed a manual duck and wrote a `Ducked` log line **unconditionally**:

```rust
clock.manual("Music", stinger.duck_db, now, stinger.duck_s, DEFAULT_FADE_S);
log.push(Entry::Ducked { at_s: now, bus: "Music", by_db: stinger.duck_db, by: "stinger" });
```

`model::active_duck_db` returns `0.0` for **any** `by_db >= 0.0` (`src/model.rs:187`), so a stinger whose `duck_db` is
exactly `0.0` — or a positive number, meant as a lift — armed an envelope that lowers nothing while the log said
`Ducked { by_db: 0.0 }`.

**Measured** (`Stinger { cue: "sting", duck_db: 0.0, duck_s: 1.0 }`, one frame): the log held
**`[Ducked { by_db: 0.0, by: "stinger" }]`** and `Music` did not move.

## Why this is rank 1 and not a report

Iteration 52 fixed the identical inert path for a **table rule** by adding a sentence to `SirenSettings::problems()`. A
`Stinger` is a **message**, not configuration: it is sent at runtime, so no author-time report can see it. `MusicScore`
has a `problems(&table)` and it does not — and cannot — examine a message that has not been sent.

So the log line was the *only* place a person could learn what a stinger did, and it said the opposite of what happened.
A duck is the whole point of a stinger: `Stinger::new` defaults to `duck_db: -6.0, hold 1.0 s`, so a `0.0` is a
project computing the depth from gameplay — the same shape as iteration 52's `DuckClock::manual` case.

## Sources

**S1. `Stinger::new(cue)`'s own doc** — *"A stinger of `cue` ducking the music by **6 dB** for a second."* The type's
name for itself is a duck; a zero depth is not a quieter stinger, it is a stinger that does nothing.

**S2. `active_duck_db`'s guard** (`src/model.rs:187`) — `if !by_db.is_finite() || by_db >= 0.0 { return 0.0 }`. Deliberate
for a raise; zero falls between "a reduction" and "a raise" and is neither.

**S3. `duck_db`'s floor** (`src/mix.rs:489`) — `deepest = 0.0`, then `if db < deepest`. Nothing at or above zero can ever
be recorded, so the envelope is inert by construction.

**S4. Iteration 52's rule fix, the same shape** — `SirenSettings::problems()` now says *"the duck on `X` lowers it by 0
dB, which is nothing at all"*. This iteration is the **message** side of that question, where no report can reach.

**S5. `arm_stinger_ducks`' own doc** — *"Arms a stinger's duck once its play is answered, and not before"*, with the
reason spelled out: *"A duck for a refused play would dip the music for a sound nobody hears"*. The same reasoning applies
one step further: a duck that cannot dip should not claim it did — it is a duck for a depth nobody hears.

## The technique

Arm and log only when the depth can actually lower the bus: `duck_db.is_finite() && duck_db < 0.0`. One condition, the
same one `active_duck_db` uses to decide whether the value means anything.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a stinger's default depth | **−6.0 dB**, hold 1.0 s | S1 |
| `active_duck_db`'s inert threshold | `by_db >= 0.0` | S2 |
| `duck_db`'s floor | `deepest = 0.0`, `if db < deepest` | S3 |
| `Ducked` lines for a `duck_db: 0.0` stinger, before | **1** — `by_db: 0.0` | measured |
| `Music`'s movement for that stinger | **none** | measured |
| `Ducked` lines after | **0** | measured |

## What good sounds like

A `-9.0` stinger dips `Music` and writes `Ducked { by_db: -9.0, by: "stinger" }` — one line, the stinger's own depth. A
`0.0` or a positive one writes nothing, because nothing happened; the stinger's own *hit* still plays, which is the part
a player hears.

## Blocked

`hs distill search` answered `500 … gpu busy` for the seventh iteration running, so the sources here are the crate's own
docs and guards plus the vendored `docs/research/siren.md`. No DOI bears on a log line that contradicts its own effect.
