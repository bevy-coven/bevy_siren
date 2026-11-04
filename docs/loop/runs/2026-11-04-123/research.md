# Iteration 123 — research: a refusal nobody prints

**Item**: `needs-decision/log-cursor.md`, option **4** — **the crate prints an authoring refusal once, on the console** —
and the measurement that file asks for first: *how many refusals does a healthy game produce?*

## The defect, in the crate's own words

`docs/guide/cues.md`: *"There is no silent failure. Every refused play is a `Refused` with a reason that names what is
missing."* The `Refused` **message** always arrives. The **log** always has it. And a game that never reads either — the
ordinary game, the one that calls `Play::cue` and moves on — learns **nothing**: `src/log.rs:7` says *"A crate that prints
is a crate a game cannot silence: the game prints the log"*, so the crate emits no line at all for a typo. Iteration 120
found the README's own quick start in exactly that state (four defects, none printed).

## The sources

| source | what it says | where |
|---|---|---|
| **Avižienis, Laprie, Randell and Landwehr**, *Basic Concepts and Taxonomy of Dependable and Secure Computing*, IEEE Trans. Dependable and Secure Computing 1(1), 2004 | A failure is classified by whether the service interface **signals** it: *"an unsignalled failure"* is one the interface does not raise. A `Refused` a game does not read is a failure in the interface's own terms. | DOI 10.1109/TDSC.2004.2 |
| **Hunt and Thomas**, *The Pragmatic Programmer*, 20th ed. | Tip 38, *crash early*: a problem should be obvious as early as possible; Tip 20, *it's all writing*: a diagnostic is a message written for a person. | ISBN 978-0135957059 |
| **Ousterhout**, *A Philosophy of Software Design*, 2nd ed. | §10.3 *define errors out of existence* — but also that an interface must not make a caller's mistake indistinguishable from correct use; the pair is cited in `AGENTS.md` for the owner of a play (iteration 119). | ISBN 978-1732102217 |
| **Uddin and Robillard**, *How API Documentation Fails*, IEEE Software 2015 | Incorrectness and incompleteness are the two severest documentation problems, and *"blockers"* made a developer abandon a unit. A refusal that exists only in a log a game filters away is a document nobody opens. | DOI 10.1109/ms.2014.80 |
| **Bevy 0.20's `bevy_log`** (`~/.cargo/registry/…/bevy_log-0.20.1`) | `warn!` is a `tracing` event: filterable **by target and by level**, at runtime (`RUST_LOG`, `LogPlugin::filter`), and silent when no subscriber is installed. A `println!` is not filterable. | vendored source |
| this crate, `src/log.rs:7` | *"Nothing here is a `println!`. A crate that prints is a crate a game cannot silence."* | the sentence to settle |
| this crate, `docs/loop/needs-decision/log-cursor.md` | option 4 was recorded as *the next candidate* with a measurement first: how many refusals a healthy game produces, so the rule can promise it prints none of them. | iteration 120 |

The technique, in the sources' terms: **a diagnostic goes where a person will see it, and only for the class of failure
that is not normal operation.** A `tracing` event satisfies both halves: it reaches the console the game's own subscriber
prints, and it is filterable — *silenceable* — which is what `src/log.rs:7` requires and what a `println!` cannot give.

## The measurement the file asks for, before any code

A healthy game's refusals, from the shipped examples and every cue case this loop runs:

- **`VoiceLimit` is normal operation**: `examples/budget` refuses one-shots on purpose once the cap is full (a cap of 32
  against 40 plays), and `cues.yaml` cases 7, 8, 9, 21 and 24 are all about the budget doing its job. **It must never
  print.** It is also the one refusal whose sentence the crate already knows is not a fault.
- **`MissingSound` can be normal for a moment**: a play while a load is in flight is *not* refused (it waits), but a
  project that reloads a table and plays immediately can meet `MissingSound` on a key that is still loading. That is why
  `sounds.failed()` exists: the crate can tell *"no handle yet"* from *"the server says it failed"*.
- **`UnknownCue`, `UnknownBus`, `NoSounds`** are **authoring** mistakes: no state of a running game produces them; only a
  table that disagrees with the code, or a typo in a name.
- The healthy examples — `quick_start`, `music`, `budget`, `mixer` in its shipped state — refuse **0** authoring
  refusals. `mixer` refuses only when asked to (`SIREN_KEYS=…,refuse`), which is the picture's point.

So the split the rule can promise: **an authoring refusal prints once; a budget refusal never prints; a `MissingSound`
prints once, and its sentence already names whether the file failed or the key is simply not loaded.**

## The rule, and what it costs a consumer

**A `warn!` with `target: "bevy_siren"` for `UnknownCue`, `NoSounds`, `MissingSound { .. }` and `UnknownBus`, and
nothing for `VoiceLimit` or `SpatiallyMute`.** Once per **cue and reason per frame**, because a burst of a hundred plays in
one frame is one mistake. `SpatiallyMute` is not printed: it is an architectural state (the crate says how to fix it)
that a game learns once from the guide, and a scene with a listener arriving a frame late would otherwise print at frame
zero of every run.

**Silenceable three ways**: `RUST_LOG=bevy_siren=off`, a `LogPlugin` filter, or having no `tracing` subscriber at all
(the crate's own headless tests). `bevy_log` is already a dependency (`Cargo.toml:120`), so the cost is **zero new
dependencies** and one formatted string per refusal.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| a diagnostic belongs on the console, filterably | a `tracing` event, target `bevy_siren` | `bevy_log-0.20.1`; `src/log.rs:7` |
| which class is a fault, which is normal operation | authoring: unknown cue, no sounds, missing sound, unknown bus. Operational: the budget | Gregory, *Game Engine Architecture* 3e §14.5.8.4 (a voice limit is a feature); `cues.yaml` 7–9, 21, 24 |
| once per what | per cue and reason per frame — a frame's hundred plays are one mistake | design default |
| the sentence | the log's own (`Entry::line` / `refusal_sentence`), so the console and the log cannot drift | design default |

## What good looks like

A project that has never read the guide, with `footstep.grss` in its code and `footstep.grass` in its file, sees on its
own console, once:

```
WARN bevy_siren: refused `footstep.grss`: no cue of that name
```

A game whose budget is full prints **nothing** — the budget is not a fault. A game that sets
`RUST_LOG=bevy_siren=off` prints nothing at all. A headless test with no subscriber prints nothing. The log's own
`Refused` entries are unchanged, byte for byte, so the panel and `report` read what they read.

## Predictions (written before the build; scored in `review.md`, wrong ones first)

1. **P1** — the unchanged tree prints **nothing** for an authoring refusal: a headless app with a subscriber captured (a
   thread-local `tracing` layer, as `tests/quick_start.rs` does) sees no line naming the cue.
2. **P2** — after: one line per **cue and reason per frame** for `UnknownCue`, `NoSounds`, `MissingSound`, `UnknownBus`,
   with target `bevy_siren`, and the sentence equal to the log's `Refused` entry.
3. **P3** — `VoiceLimit` prints **nothing**, even at 40 plays against a cap of 2.
4. **P4** — `SpatiallyMute` prints nothing.
5. **P5** — a hundred `UnknownCue` plays in one frame print **one** line, and two different unknown cues print **two**.
6. **P6** — no subscriber (the headless tests' ordinary app) prints nothing and does not panic; `bevy_log`'s default
   filter is not needed for the crate to work.
7. **P7** — the log's `Refused` entry and the line are the same sentence, and the `Refused` message is still written.
8. **P8** — the quick start's console is **unchanged** when its cues are right (0 lines), and the negative control (the old
   keys) gains **one** line per key, which is exactly the mistake iteration 120 found.
9. **P9** — the public surface gains nothing: no new field, method, message or schema; the only addition is the console
   line, and `Cargo.toml` already has `bevy_log`.

## Carried off, not built

- A **setting** (`SirenSettings::print_refusals`) — a schema change the editor reads; `RUST_LOG` already does it.
- **Printing `SpatiallyMute`** — an architectural state, and a scene whose listener arrives late would print at frame
  zero of every run.
- The carried top-5 items (the release, a moving owner's level, score layers starting together, a person listening).
