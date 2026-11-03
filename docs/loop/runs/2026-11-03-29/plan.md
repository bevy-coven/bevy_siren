# plan.md — iteration 29: the README's pictures are the panel the code draws

## The pick, and why it beats the runner-up

**Pick:** the README — the crate's front door — showed two images of `examples/mixer` generated **before** iteration 26
added the `Problems` block, described in alt text that names *"eight cues … eight log lines"* when the shipped table has
**nine** cues and the log shows **ten**. A third media file, created by iteration 26, was named in **no** README.

Beats the runner-up (a `Stopped` line for a parent's despawn): that one is blocked on the public-enum decision six items
already wait on. This one is unblocked, it is the first thing a developer sees, and it is a **false** picture rather than
a missing line.

## The design

1. **Regenerate `docs/media/mixer.png`** from the clean table — the state a project ships, `Problems / none`.
2. **Regenerate `docs/media/mixer.gif`** with `SIREN_FAULTS=1`, so the `Problems` block and the `<- problem` markers are
   visible through the animation.
3. **Delete `docs/media/mixer-panel.gif`**, the orphan from iteration 26.
4. **Rewrite both alt texts from the regenerated pictures** — counts read off the image, not remembered — and the GIF's
   alt describes the animation rather than one frame.
5. **`docs/media/README.md`** documents `mixer.gif` and `mixer.png` with both regeneration commands; no dangling name.

## Fields

None. No `src/` change, no public type, no schema; `cargo test` counts are unchanged (112 / 119 / 121 + 5).

## Tests

The crate does not ship `docs/`, so the suite cannot see an image. What proves this:

- **`check_media.py`** — every image a README names exists; every image in `docs/media` is named by a README. It passes on
  the fixed tree (6 ok, exit 0) and **reports the orphan and exits 1** on a scratch reproduction of the pre-fix state.
- **Both images looked at** with `read`, and every count in the alt text checked against the picture.
- The full gate, unchanged.

## The carve

- **Not** a test in `tests/`: `docs/` is not shipped, and a test that reads the repo root would be a test of the checkout
  rather than the crate. The script lives in the run directory and is named in `log.md` for the next run.
- **Not** the skill's "MSRV 1.98.1" note: the crate's `rust-version = "1.97.1"`, Bevy 0.20.1's own `rust-version` is
  `1.97.1`, and the README says MSRV 1.97.1 — all three agree, and the note is wrong for this dependency. Recorded in
  `log.md`, not edited.
