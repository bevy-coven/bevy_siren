# review.md — iteration 43, three passes

## Pass 1 — Function: does it do what the source says?

**The source.** S1: `Play::on` parents the voice, *"as a game does for an engine on its vehicle"* — the owner's despawn is
its ordinary end. S2: every other end writes a `Stopped` line. S5 (R6): *"everything a play decides is readable"*.

**Measured on the panel** (`evidence/panel/parent-90.png`, 1280×720, frame 90, default script):

| Where | Text |
|---|---|
| log | `0.67s played \`engine.idle\` (engine_loop) on Effects at -14.00 dB` |
| log | `0.68s stopped \`engine.idle\`: owner despawned` |
| bus row | `Effects +0.0 dB 1 / 2 voices` |
| header | `3 voices` (was 4 with that voice alive) |

The log and the mix agree: the sound stopped, it says which sound and why, and the count fell by one.

**The guard that matters.** `tests/play.rs::a_stop_for_an_ended_voice_is_a_no_op` — a game despawning a voice it never
parented — reddened on the first draft and passes now, because the line is gated on `ChildOf`. That is the difference
between "a sound ended" and "a sound *lost its owner*".

**Verdict: GREAT.** The one unlogged end is logged, and the one that must stay silent does.

## Pass 2 — Zoom: read the crops closely

Read: `evidence/panel/{parent-80,parent-90,parent-only,zoom-parent-log,zoom-parent-only}.png`.

**A defect found and fixed by looking.** The first `parent-80.png` showed `played \`engine.idle\`` with **no stop line**:
the `owners.retain` that despawns the owner had been inserted *after* `stage`'s `since % every != 0` early return, so it
only ran on a step frame — and the owner was therefore never dropped before the capture. Moving the `retain` above the
gate fixed it. The capture is what caught this; no test would have.

**A second reading, checked against the timestamps.** The full-script still shows `0.69s stopped \`engine.idle\`: owner
despawned` **above** `0.67s played \`engine.idle\``, which looks like a stop preceding its own play. It is not: read
bottom-up (oldest first) the timestamps descend monotonically — `0.34` play `sword.hit`, `0.51` steal of `engine.idle` by
`bang`, `0.51` play `bang`, `0.57` `sword.hit` ends, `0.67` play `engine.idle` on its owner, `0.69` that owner despawned.
Every stop follows its own play, and the isolated `SIREN_KEYS=parent` run (`zoom-parent-only.png`) repeats the pattern
three times: `0.32 → 0.34`, `0.57 → 0.59`, `0.83 → 0.85`. Noted because a reader can misread a newest-first log, and a
capture that invites that misreading is worth the sentence.

**No clipping, no gap.** `parent-90.png` keeps a 9 px margin and the whole `Problems` and log are inside the frame.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: a different script and a different table

- `SIREN_KEYS=parent`, `SIREN_KEY_EVERY=30` — three spawn/despawn cycles, each one `played` and one
  `owner despawned`, in the right order.
- `SIREN_FAULTS=many` — 14 cues, unchanged layout.
- The **mixer** backend: `a_despawned_voices_mixer_slot_is_freed` asserts the slot is freed **and** that exactly one
  line exists, which is what caught the double-log in the first draft.

**Can a developer understand it without the code?** Yes. `owner despawned` names the mechanism in the same breath as
`ended`, `stop` and `stolen by …`, and `docs/guide/buses.md` now lists it with the others. Nothing needs the source.

**Verdict: GREAT.**
