# Should `SirenLog` record that a duck *finished*?

**Status:** open — found and sized in iteration 27; not built.

## The problem

`Entry::Ducked` is written when a duck **starts**, from `play.rs` (a cue that ducks on start) and `music.rs` (a stinger).
Nothing is written when it **ends**. Measured, the envelope is:

```
dialogue.greeting on Dialogue, default settings, 60 fps
  frame  0: -8.0   10: -8.0   15: -7.9   20: -6.9   30: -4.7   40: -2.6   50: -0.5   55: +0.0
```

So a panel drawing `SirenLog` shows *"dialogue.greeting ducked Music by −8.0 dB"* and never *"Music recovered"*. A reader
sees the dip and not the release. The **state** is observable — `SirenMix::ducked` returns `0.0` when a bus is free, and
the panel's own `duck` column returns to `+0.0` — so this is a **log** gap, not a missing fact.

## Options

### A — `Entry::Recovered { at_s, bus }`

One line when a bus's duck reaches `0.0` after having been negative.

- **Cost:** a **new variant of a public enum without `#[non_exhaustive]`** — the same break as the four `CueProblem`
  variants already waiting. A consumer matching `Entry` exhaustively must add an arm.
- **Benefit:** the log tells the whole story of a duck, which is what *"the log of what played and why"* claims.

### B — nothing; a host watches `SirenMix::ducked`

- **Cost:** a host that only reads `SirenLog` (a text panel, a CI diff of a capture) cannot see the release. The crate
  already exposes the fact; the log does not repeat it.
- **Benefit:** no API change, and no line per duck — a busy scene ducks often, and one line per release doubles the duck
  traffic in a 256-entry log.

### C — a `depth` on the existing `Ducked` entry, and no release line

- **Cost:** also a breaking change (a field on a variant breaks every construction **and** pattern), and it still does
  not say *when* it ended.
- **Benefit:** none over B.

## Recommendation

**B, with a note in `Entry::Ducked`'s doc** that the release is `SirenMix::ducked`, not a log line. The log's 256 entries
are the scarce resource (iteration 24 made refusals outlive plays precisely because they are exceptional); a duck release
is as routine as the duck itself and would double that traffic for a fact the crate already publishes every frame.

**What it costs a consumer today:** a text panel shows the dip and not the release; the panel's own `duck` column shows
both. `docs/guide/buses.md` already tells a host to read `deepest_duck()`.

## What this iteration does instead

Nothing in `src/`. The **doc gap is real and cheap**: `Entry::Ducked`'s doc does not say where the release lives, so a
host reading it concludes the log is the whole story. That sentence is added — no API change.
