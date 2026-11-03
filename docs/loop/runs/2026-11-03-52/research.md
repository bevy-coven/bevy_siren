# research.md — iteration 52: a duck that lowers nothing says so

## The item

Iteration 51's top-5 listed two reports blocked on `needs-decision/bad-gain-problem.md`, because `CueProblem` is a public
enum without `#[non_exhaustive]`. **This iteration's item is not one of them**, and probing found why that distinction
matters:

`SirenSettings::problems()` returns **`Vec<String>`**, not `Vec<CueProblem>` (`src/mix.rs:143`). A fault in the *mix
table* is a sentence, so reporting one is **additive** — exactly the shape of iteration 48's bus-cap sentence. The enum
gate covers cue-table faults only.

**Measured** (`SirenSettings::problems()`, one duck, `max_voices: 32`):

| duck | `problems()` |
|---|---|
| `by_db: 0.0`, hold 0.25, fade 0.6 | **`[]`** |
| `by_db: NaN`, hold 0.25, fade 0.6 | **`[]`** |
| `by_db: +6.0` (a raise) | 1 sentence — already reported |
| `by_db: -8.0`, no hold, no fade | 1 sentence — already reported |

`active_duck_db` returns `0.0` for **any** `by_db >= 0.0` (`src/model.rs:187`), and `duck_db` only keeps a reduction
`if db < deepest` with `deepest` starting at `0.0` (`src/mix.rs:489`). So a rule at `by_db: 0.0` fires, is recorded in
the log, holds its `hold_s`, and lowers the bus by **nothing at all**.

## Reachability, measured

**The RON path refuses `nan` outright.** Parsing
`(ducks: [(source: "Dialogue", target: "Music", by_db: nan, ...)])` answers
`REFUSED: the mix table does not parse: 1:70-1:71: Expected float`. So a non-finite `by_db` is **not authorable** and
needs no sentence.

**`by_db: 0.0` parses.** The same text with `0.0` answers `parsed, problems: []`. That is the authorable silent case.

**The runtime path is `DuckClock::manual(target, by_db, now_s, hold_s, fade_s)`** (`src/mix.rs:467`) — where a game
computes a duck depth from gameplay (distance, health, an intensity curve) and can pass `0.0` at the bottom of its range.

## Sources

**S1. Jason Gregory, *Game Engine Architecture*, 3rd ed. §14.6.3** — the dialogue duck every game wants: **−8 dB** on
music, **−4 dB** on effects, hold **0.25 s**, fade **0.6 / 0.4 s** (`docs/research/siren.md`, the parameter table). A
duck is a *reduction*; a `by_db` of `0.0` is not a gentler duck, it is the absence of one.

**S2. The same chapter on grouping** — *"sounds are grouped into categories or buses, and the mix is controlled by
adjusting the level of each bus"* (`docs/research/siren.md:103`). A bus level that a rule claims to lower and does not is
the one thing the grouping model cannot express.

**S3. `active_duck_db`'s own guard** (`src/model.rs:187`) — `if !by_db.is_finite() || by_db >= 0.0 { return 0.0 }`. The
zero is deliberate for a raise; the report already names a raise (`by_db > 0.0`). Zero itself falls between the two
conditions.

**S4. `duck_db`** (`src/mix.rs:480-499`) — `deepest = 0.0`, then `if db < deepest`. Nothing at or above zero can ever be
recorded, so the rule is inert by construction, not by rounding.

**S5. Existing report vocabulary** — `SirenSettings::problems()` already says *"the duck on `X` has no hold and no fade,
so it lowers the bus for no time at all"* for the neighbouring fault. A rule that lowers by zero is the **same sentence
with a different cause**, and the crate's rule is one path per fault.

## The technique

Add one sentence to `SirenSettings::problems()`: a duck whose `by_db` is exactly `0.0` lowers nothing, whatever its hold
and fade. It is not a new `Refusal` and not a new `CueProblem`; it is the mix table's existing report gaining the case it
was missing.

**Why not also report `NaN`:** RON refuses it at parse time, so an author cannot write it; a `Duck` built in code with a
non-finite `by_db` is already covered by the crate's documented rule that a non-finite number is neutralised
(`finite_or`, and `active_duck_db`'s own guard). Reporting it would be a second sentence for a fault the parser prevents.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a dialogue duck's depth on music | **−8 dB** | Gregory §14.6.3 |
| on effects | **−4 dB** | Gregory §14.6.3 |
| hold | **0.25 s** | Gregory §14.6.3 |
| fade (music / effects) | **0.6 / 0.4 s** | Gregory §14.6.3 |
| `active_duck_db`'s inert threshold | `by_db >= 0.0` | S3 |
| `duck_db`'s floor | `deepest = 0.0`, `if db < deepest` | S4 |
| RON's handling of `nan` | **refused at parse** | measured |
| reports for `by_db: 0.0` before | **0** | measured |

## What good sounds like

`SirenSettings::problems()` naming the rule once — *"the duck on `Music` lowers it by 0 dB, which is nothing at all"* —
while the crate's own `default_ducks()` stays silent, because −8 dB with a 0.25 s hold and 0.6 s fade is a real duck.
On the panel, a mix table with an accidental `0.0` shows a problem line instead of a rule that looks configured and does
nothing.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`**,
the same refusal as iteration 51. The numbers here are the vendored `docs/research/siren.md`, which records Gregory's
§14.6.3 table; no new DOI was needed for a fault proved by the crate's own guards.
