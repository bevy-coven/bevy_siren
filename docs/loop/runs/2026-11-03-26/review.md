# review.md — iteration 26, three passes

## Pass 1 — function

**The claim.** `examples/mixer`'s panel is the crate's visible harness, and its module doc says it shows *"the cue
table"*. It showed the cue's name, bus, mode, gain and priority — and **not** the crate's own verdict on them. A cue on a
bus the mix does not have was drawn exactly like a healthy one, so the picture said a broken cue was fine. The Cauldron's
`audio_panel.rs` *does* chain `problems_in(&read)`; the crate's own example did not.

| row | source | before | after |
|---|---|---|---|
| a cue on `Reverb`, a bus the table lacks | S1 (the panel shows the cue table), S2 (`CueProblem`'s sentence exists to be printed) | drawn as a healthy row, **nothing marked** | row ends `<- problem`; `Problems` block says why |
| a cue with `pitch.low > pitch.high` | S2 | same | same treatment |
| a clean table | S3 (*"can a developer understand the panel without the code?"*) | no `Problems` section at all | `Problems` / `none`, and no row marked |
| every existing row | — | unchanged | unchanged |

**Against the sources.** S2 is the whole argument: the crate computes the sentence (`CueProblem::Display`) and the panel
never asked for it. S3 is what the panel is for — a view that cannot distinguish a playable cue from an unplayable one
fails the one question the skill asks of it.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{clean-1280,faults-1280,zoom-problems-clean,zoom-problems-faults}.png`, and the regenerated
`docs/media/mixer-panel.gif`.

**A defect in my own first version, caught by looking.** The row marker was the **full `CueProblem` sentence**, and at
1280 px it wrapped onto a second line, breaking the cue column's alignment and repeating what the `Problems` block below
already said:

```
sword.hit.wrong-bus  Reverb  once  +0.0 dB  priority 0  <- cue `sword.hit.wrong-bus` plays on bus `Reverb`, which
the mix table does not have
```

Two forms of one idea, and a broken table shape. The marker is now the three words `<- problem`; the sentence lives only
in the block. Re-captured, the column is aligned and each idea has one home.

**The clean table** reads `Problems` / `none` with no markers — so the block is not noise on a healthy project.

**The `problems_in` choice matters here and is visible in the picture:** the example's table uses `Ambience` and `Music`,
which are **not** the crate's default four. `problems()` would have printed five false `UnknownBus` sentences on the clean
capture. `problems_in(&settings)` prints `none`. Iteration 21's fix is load-bearing for this panel.

**Verdict: GREAT** after the marker fix (pass 1 re-ran).

## Pass 3 — fresh eyes

**A different shape.** Two captures of one binary, differing only in `SIREN_FAULTS`: the clean table proves the block says
`none`, and the fault table proves it says something and marks the rows. The fault table is **capture-only** — the shipped
screenshot stays clean, because it is the crate's face.

**Can a developer understand the panel without the code?** Yes, and now more so than before: the panel's four blocks are
header, buses, cues **with markers**, the crate's verdict, and the log. A person sees that
`sword.hit.wrong-bus` plays on `Reverb`, that `Reverb` is not a bus in the list above, and that the crate refuses it —
three places agreeing, no code needed.

**What a fresh reader would still ask:** *why is the marker a word and not a colour?* Because the panel is monospace text
with no style vocabulary, and a word survives a PNG at 1280 px where a colour shift does not.

**Verdict: GREAT.**
