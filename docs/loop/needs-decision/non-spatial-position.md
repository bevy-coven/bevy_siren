# Should a non-spatial cue given a position be refused?

## The question

A cue authored `spatial: false`, played with `.at(position)`, now **ignores** the position: it plays flat, unpanned, at
the gain `Played` reports. Should the crate instead **refuse** it, as it refuses a spatial play with no listener?

## The options

1. **Ignore it, and say nothing** (shipped in iteration 8). Cost: none. A project that passed a position by mistake gets
   a flat sound and a true `Played.gain_db`, but no line saying the position did nothing. Benefit: no public change;
   `spatial: false` means what it says.
2. **Ignore it and log it** — a line in `SirenLog` beside the `Played` entry. Cost: one new `Entry` variant, which is a
   public enum change for any consumer's exhaustive `match` (the Cauldron renders `Entry::line()`, so it needs an arm or
   a wildcard). Benefit: the mistake is visible in the panel without a break in `Refusal`.
3. **Refuse it: `Refusal::NotSpatial(position)`**, printed *"cue `ui.click` is not spatial, so a position at [0, 0, 8] has
   no meaning"*. Cost: a **new `Refusal` variant** — an exhaustive `match` in a consumer breaks, and a game that passed a
   position speculatively for a cue that may or may not be spatial now gets nothing at all instead of a flat sound.
   Benefit: the same strictness the crate already applies to "spatial play, no ears", argued by the same sentence in
   `docs/guide/spatial.md`.
4. **Refuse only when the position differs from the listener's** — i.e. treat "at my feet" as the same as "flat". Cost:
   the same break as 3, plus a comparison that makes behaviour depend on where the listener happens to be. Benefit: none
   worth the confusion; not recommended.

## The recommendation

**Option 2 in the next minor version, and option 3 never.** The asymmetry matters: a spatial play with no listener
cannot be honoured *at all* (the crate has no place to put it), so refusing is the only honest answer. A non-spatial cue
with a position **can** be honoured — as flat, which is what the cue says it is — so refusing would throw away a sound
the game asked for. Option 2 keeps the play and makes the ignored input visible, which is what the loop's rule asks for;
its cost is one `Entry` arm, the same cost `remove-completes.md` already trades.

## What is blocked

Nothing: iteration 8 shipped option 1, which needs no answer and no public change. Option 2 waits on the `Entry` decision
it shares with `remove-completes.md`, and can land with it.
