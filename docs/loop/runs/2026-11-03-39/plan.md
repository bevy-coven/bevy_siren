# plan.md — iteration 39: the panel shows a refusal

## The pick, and why it beats the runner-up

**Pick:** the panel's claim is *"the log of what played and why"*, and the scripted run **never refused anything** — so the
`Log` block had never drawn a `Refused` line. The skill ranks a silent refusal as the most valuable bug; the same logic
makes it the most valuable **line**, and the panel that exists to answer *"why is the music quiet"* had never answered
*"why is there no sound at all"*.

Beats the runner-up (a `Stopped` line for a parent's despawn): that is blocked on the public-enum decision seven items wait
on. This one needs no `src/` change and shows a message the crate already writes.

## The design

1. **`Step::Refuse`** — `Play::cue("no.such.cue")`, giving `Refusal::UnknownCue`.
2. **In the default script**, after `duck`: `steal,layer,duck,refuse`.
3. **The log's line count follows the `Problems` block** — 7 when clean, **4** when the table has faults, because the
   fault table costs three lines (two cue rows, two problem sentences) and seven leaves `1 px` of margin.

## Fields

None. Example only; no `src/` change, no public type, no schema.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **The still, read:** its log opens with `0.83s refused \`no.such.cue\`: no cue of that name`, at frame 95.
- **The fault table at 1280, measured:** four log lines, lowest ink `y = 709`, margin **11 px** — no clip. Seven and five
  both leave `1 px`.
- **The GIF at native 1280** (it was 960 wide, which wrapped nothing but wasted pixels): every block legible, no clip.
  Its first frame is at **frame 88, before the refusal**, so it shows `played` lines — recorded, not hidden.
- `check_media.py`: 0 problems.

## The carve

- **Not** a `Refused` counter in the header: the log shows the line, and a count would be a second place the same fact
  lives.
- **Not** a longer log on the fault table: measured, four is what fits; the faults are what the reader came for.
- **Not** re-timing the GIF to land on the refusal: `SIREN_SHOT_FROM` is a knob, and the capture is a sample of a longer
  run — the still proves the line, the GIF proves the layout.
