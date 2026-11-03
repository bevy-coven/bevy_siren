# research.md — iteration 53: `MusicScore::default()` is the score the file loader gives

## The item

`MusicScore` derived `Default`, so `beats_per_bar` was **`0.0`**. The field also carried
`#[cfg_attr(feature = "serde", serde(default = "MusicScore::default_beats_per_bar"))]`, whose value is **`4.0`**. Two
spellings of one field, and `Default` had the wrong one.

**Measured**, the same score written both ways:

| path | `beats_per_bar` | `bar_s()` | `problems()` |
|---|---|---|---|
| from RON, key omitted | **4.0** | `Some(2.0)` | `[]` |
| `MusicScore::default()` | **0.0** | **`None`** | **`["beats_per_bar is not a positive number"]`** |

So the crate's own default produced a score **the crate's own report called broken**, and one with **no beat grid at
all** — every `NextBeat` and `NextBar` change landing immediately instead of quantised.

## Why this is rank 1, not rank 3

It is not a missing sentence; it is **two different scores from one struct**. A project that builds its score in code
(an example, a test, a game generating a score at startup) gets a different beat grid from one that ships a `.ron` — and
the only symptom is that music transitions do not land on the bar.

The crate's own rule is *"the two backends answer identically"* and *"a second spawn path is a second set of rules"*. A
second **default** is the same fault in a different place: `tests/leaf.rs` exists to catch exactly this class, and a
`Default` nobody checks is where it hides.

## Sources

**S1. `MusicScore::default_beats_per_bar()` — `4.0`**, documented *"Four beats to the bar"*, and used by serde's
field-level default. The crate's own answer.

**S2. `MusicScore::bar_s()`** — `Some(f64::from(60.0 / tempo_bpm) * f64::from(beats_per_bar))`, with `None` when
`beats_per_bar <= 0.0`. A bar of zero beats is not a fast bar; it is **no grid**, which `None` says correctly and which
`problems()` reports correctly. The fault was never in `bar_s` — it was in the number handed to it.

**S3. `docs/research/siren.md` R5 — layering lands on the bar.** *"A change is quantised to the next beat or bar from
tempo alone (`next_boundary`), so no beat map is authored"*, proved by `tests/model.rs::the_bar_is_two_seconds_at_120_bpm`
and `tests/music.rs::a_change_waits_for_the_bar`. At 120 bpm and four beats, a `NextBar` change from 0.3 s lands at
**2.0 s**, and no gain moves before it. With `beats_per_bar: 0.0` no such guarantee exists.

**S4. Khan et al. 2023, *Adaptive Background Music for a Fighting Game*, arXiv:2303.15734** — vertical layering is
**per-instrument gain keyed to game state**, *"the stems of a single piece"* with *"the same tempo and beat throughout"*.
A score with no bar cannot hold that invariant.

**S5. The crate's own precedent** — `SirenSettings::problems()` reports a `max_voices` of `0` as *"no sound can ever
play"*. A `beats_per_bar` of `0.0` is the same shape: a number that disables a feature while looking like configuration.

## The technique

Write `Default` by hand so the field has **one** value, from `default_beats_per_bar()`, and keep serde's field default
pointing at the same function. The two spellings become one.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| beats in a bar, default | **4.0** | S1 |
| beats in a bar, from a file that omits it | **4.0** | S1 (serde field default) |
| beats in a bar, from `derive(Default)` before | **0.0** | measured |
| `bar_s()` at 120 bpm and 4 beats | **`Some(2.0)`** | S2, S3 |
| `bar_s()` at 0 beats | `None` — no grid, not a fast bar | S2 |
| a `NextBar` change from 0.3 s at 120/4 | lands at **2.0 s** | S3 |

## What good sounds like

The same score either way: `beats_per_bar == 4.0`, `bar_s() == Some(2.0)` at 120 bpm, `problems()` silent, and a
`NextBar` transition that waits for the bar instead of landing on the frame it was asked for.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`**,
the third iteration in a row. The numbers here are the vendored `docs/research/siren.md`, which records R5 and Khan et
al.; no new DOI was needed for a fault proved by comparing two paths through the crate's own code.
