# research.md — iteration 29: the front door shows a panel that no longer exists

## The item

`README.md` is the crate's front door, and it carries **two images of `examples/mixer`**. Both were stale by the time this
iteration began, and one media file was orphaned:

```
docs/media/mixer.png    generated 2026-11-02 14:40 — BEFORE iteration 26's Problems block
  the still shows 9 cues and NO `Problems` section; the panel has had one since iteration 26
docs/media/mixer.gif    generated 2026-11-02 14:40 — same
docs/media/mixer-panel.gif   created by iteration 26, named in NO README   <- orphan
```

And the README's alt text described the **old** panel in numbers a reader can check against the picture:

> *"…the **eight** cues with their buses, modes and priorities, and **eight** log lines"*

The shipped table has **nine** cues and the log shows **ten** lines. A developer's first look at this crate was a
picture of a panel the code no longer draws, described in counts the picture does not show.

## Sources

**S1. `AGENTS.md`, *"a feature is not done until its README line and, if visible, its GIF exist"*** — and this repository's
own documentation rule points at `bevy-coven/.github/DOCUMENTATION.md`. A GIF that exists but is stale satisfies the
letter and not the rule.

**S2. The skill's step 7** — *"A visible change also ships its picture … listed in `docs/media/README.md` with the command
that regenerates it."* Iterations 25 and 26 changed exactly what the panel draws, regenerated a **third** file
(`mixer-panel.gif`), and left the two the README points at untouched. The commands were right; the **target** was not.

**S3. `docs/media/README.md`** — it documents `mixer-panel.gif` and the command that writes it. After the deletion below it
documents `mixer.gif` and `mixer.png` with both commands, so there is **one** name per artefact.

**S4. Nothing in `cargo test` reads `docs/`.** The crate does not ship it, so the suite cannot catch a stale or missing
image. That is why this survived 26 iterations of a loop whose step 6 is "look at the panel".

## The technique

1. **Regenerate both images the README points at.** The still from the clean table (`Problems / none` — what a project
   ships); the GIF with `SIREN_FAULTS=1`, so the `Problems` block and the `<- problem` markers are visible through the
   animation.
2. **Delete the orphan** `mixer-panel.gif` — it was created by iteration 26 and never named by a README, and keeping two
   files that show the same panel is how one of them goes stale.
3. **Rewrite both alt texts from the picture.** The still's counts (`nine` cues, `Problems / none`, the four bus levels)
   are read off the regenerated image, not remembered. The GIF's alt describes the **animation** — a duck firing, a steal,
   a layer on the bar — plus the `SIREN_FAULTS=1` table beneath it, because a frame of an animation shows one moment.
4. **Keep a checker.** `runs/2026-11-03-29/check_media.py` asserts that every image a README names exists, and that every
   image in `docs/media` is named by a README. Run against the pre-fix state it reports the orphan and exits 1.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a visible feature ships its picture, listed with its regeneration command | — | S1, S2 |
| one name per artefact in `docs/media/README.md` | — | S3 |
| the suite cannot see `docs/` | — | S4 |

## What good sounds like

A reader who opens the README sees the panel the code draws **today**: nine cues, `Problems` reading `none`, four bus rows
with their ducks and live voice counts; beside it the animated capture showing a duck fire, a voice be stolen and a layer
enter, with two deliberately-broken cues marked so the `Problems` block has something to say. No sound changes.
