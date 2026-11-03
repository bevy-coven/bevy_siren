# review.md — iteration 29, three passes

## Pass 1 — function

**The claim.** The README carries two images of `examples/mixer`, and both were generated **before** iteration 26 gave the
panel a `Problems` block. Its alt text names *"eight cues … eight log lines"*; the shipped table has **nine** cues and the
log shows **ten** lines. A third media file, created by iteration 26, was named in **no** README.

| artefact | source | before | after |
|---|---|---|---|
| `docs/media/mixer.png` | S1 (`AGENTS.md`: a feature is not done until its README line and GIF exist), S2 (step 7: the picture and its command) | generated **before** the `Problems` block; alt text counts wrong | regenerated from the clean table: nine cues, **`Problems` / `none`**; alt text read off the picture |
| `docs/media/mixer.gif` | S2 | same date, same staleness | regenerated with `SIREN_FAULTS=1`: two rows `<- problem`, the block naming both; alt describes the **animation** |
| `docs/media/mixer-panel.gif` | S3 (one name per artefact) | **orphan** — created by iteration 26, named in no README | deleted |
| every media reference | S3 | one dangling name after the deletion | none: `check_media.py` reports 6 ok, exit 0 |

**Against the sources.** S1's rule is *"a feature is not done until its README line and, if visible, its GIF exist"* — a
stale GIF satisfies the letter and defeats the point. S4 of the research is the reason it survived: **nothing in
`cargo test` reads `docs/`**, so 26 iterations of a loop whose step 6 is "look at the panel" never looked at the picture
the README shows.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** both regenerated images, the pre-fix `mixer.png` (kept as evidence), and the pre-fix `mixer.gif`.

**The stale still, seen:** nine cue rows, four bus rows, a `Log` section — and **no `Problems` heading anywhere**. The
regenerated one has `Problems` between the cue list and the log. That is the defect in one comparison.

**A defect in my own alt text, caught by looking at the GIF.** My first replacement said *"Dialogue ducking Music by 8 dB
… with `<- problem` beside the two cues"* — but the GIF's **first frame** shows `Music -2.0 dB`, no duck, because the
animation starts at frame 30 before the duck fires. Alt text for an animation must describe what **happens**, not one
frame's state; it now does, and separately says the fault table is what the markers come from.

**The check, proved both ways.** `check_media.py` passes on the fixed tree (6 ok, exit 0) and on a scratch reproduction of
the pre-fix state reports `ORPH docs/media/mixer-panel.gif` and exits 1. A check that only ever passes proves nothing.

**Verdict: GREAT** after the alt-text correction.

## Pass 3 — fresh eyes

**A different shape.** Three artefacts, three different staleness tests: the still is read for a **section that is
missing**, the GIF for a **row marker**, and the set of files for one that is **named nowhere**. The still is the clean
panel and the GIF is the fault table — deliberately different, because a project ships the clean one and the markers need
something to point at.

**Can a developer understand the crate from the front door?** Yes, and now truthfully: the still shows the state a
project ships with `Problems / none`; the GIF shows the panel **working** — a duck, a steal, a layer — and the same panel
**catching two mistakes**. That is the crate's pitch in two images.

**What a fresh reader would still ask:** *why is one image a still and one animated?* Because the README's prose does two
jobs — "here is the panel" and "here is what it does" — and one image cannot do both without motion. `docs/media/README.md`
says which is which and gives both commands.

**Verdict: GREAT.**
