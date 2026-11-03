# plan.md — iteration 40: the bus row names the duck whose depth it shows, and a stinger gets played

## The pick, and why it beats the runner-up

**Pick:** two things, one commit, because the second was **found by** the first.

1. **`Stinger` was never written.** `music.stinger` is in the example's table and no step plays it, so the panel had never
   shown the crate's second music feature — and the one duck whose `by` is a mechanism (`"stinger"`) rather than a cue.
2. **The bus row named the wrong duck.** It printed the **deepest** depth (`mix.ducked`) beside the **newest** entry's
   cause. One duck hid it; a stinger over a dialogue line exposes it: `duck -8.0 dB by stinger`, where −8.0 is
   `dialogue.greeting`'s. Two numbers from different sources presented as one fact.

Beats the runner-up (a `Stopped` line for a parent's despawn): that is blocked on the public-enum decision seven items wait
on. Both parts here need no `src/` change.

## The design

1. `Step::Stinger` → `Stinger::new("music.stinger")` (−6 dB, 1 s — the crate's documented default), in the default script.
2. `Kind::Bus` finds the newest `Ducked` for the bus and compares its `by_db` with `mix.ducked`:
   within 0.5 dB → `duck -4.0 dB by dialogue.greeting`; otherwise → `duck -8.0 dB (newest: stinger at -6.0)`; no entry → the
   depth alone.

## Fields

None. Example only; no `src/` change, no public type, no schema.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **The still, read:** `Music -9.9 dB duck -8.0 dB (newest: stinger at -6.0) 3 / 12 voices`, with the log's top two lines
  `0.84s stinger ducked Music by -6.0 dB` and `0.84s played \`music.stinger\` …`, and `dialogue.greeting ducked Music by
  -8.0 dB` two lines below — three places agreeing.
- **The single-duck case is unchanged:** `Effects -3.9 dB duck -4.0 dB by dialogue.greeting` — the short form, because
  there the newest *is* the deepest.
- **No wrap:** the bus bands are evenly spaced (196, 227, 260, 292) and the row spans x 24..817 of 1280.
- **The GIF opens on the stinger frame**, so the two-duck row is the first thing it shows.
- `check_media.py`: 0 problems.

## The carve

- **Not** a `Stinger` row in the header: the bus row already says it, and a header that repeats a row is noise.
- **Not** a "deepest" arrow on the row: two duck depths on one line is what the log is for; the row names the deepest's
  value and the newest's cause, which is the pair a reader can check.
