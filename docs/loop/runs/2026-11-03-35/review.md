# review.md — iteration 35, three passes

## Pass 1 — function

**The claim.** `MusicScore::problems`'s doc named **four** checks. The function performs **nine**. The two this loop added — a
gain that is not a number (iteration 19) and a layer on the wrong bus (iteration 34) — were never added to the doc, so a
reader trusted a list and stopped. `MusicLayer::cue`'s doc had the mirror-image defect: it still said *"should"* after
iteration 34 made that a **refusal**.

| row | source | before | after |
|---|---|---|---|
| the report's doc | S1 (the doc's own list, counted against the nine `out.push` sites) | **four** of nine | **nine**, each a clause, plus the note that the plugin enforces the last two |
| `MusicLayer::cue`'s doc | S2 (a doc that says *"should"* where the code **does**) | *"It should name the `Music` bus"* | *"It **must**"* — and what happens otherwise |
| the nine checks themselves | S3 (R6: everything a play decides is readable) | all nine present, undocumented | unchanged, now described |

**Against the sources.** S2 is the sharpest: iteration 34 fixed *"should" where the code did nothing*; this is *"should" where
the code **does***. Both are the doc and the mechanism disagreeing, in opposite directions, and both leave a reader with a
false picture — of what is caught, or of what is enforced.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/nine-checks.txt`, the spectrograms, the loudness table, one perf run.

**The nine, enumerated from one score** — the evidence file prints them numbered, so a reviewer can count without reading
`src/`:

```
1. tempo_bpm is negative or not a number
2. beats_per_bar is not a positive number
3. a music layer has no name
4. music layer `calm` is listed more than once
5. music layer `nan` has a gain that is not a number
6. music layer `missing` names cue `no.such`, which the table does not have
7. music layer `oneshot` names cue `once`, which is not a loop, so it cannot hold a layer
8. music layer `wrongbus` names cue `on.effects`, which plays on bus `Effects`; a layer is a stem of the `Music` bus
9. the score has no layers
```

**A gate failure in my own doc, caught by `RUSTDOCFLAGS="-D warnings"`:** `[`SirenLog`](crate::SirenLog)` — a redundant explicit
target, because the label already resolves. `[`SirenLog`]` alone. The gate is why the doc build is part of the baseline.

**A probe that lied, caught rather than shipped:** my first sweep row read *"layer cue on another bus → 0 problems"*, because
the cue I used was **on `Music`** — the label was wrong, not the code. Re-probed with `on.effects` and got the sentence.

**Verdict: GREAT** after the link fix.

## Pass 3 — fresh eyes

**A different shape.** The sweep was `grep every 'should' in src/` — five hits, **two** real. The rest are comments
describing history (`"the seed is the mix table's"`, `"problem names should be one word"`), true as written. The **absolute
claims** (`never`, `must`, `only`) were tested too: an empty cue **only refuses**, and a budget of 2 with six unstealable
requests plays **2, refuses 4, never 3**. Both hold.

**Can a developer understand it without the code?** The report's doc now lists what the report does; `MusicLayer::cue` says
what is enforced. A person reading either learns the crate's rules without opening a function.

**What a fresh reader would still ask:** *why not report a gain above 0 dB?* Because a boost is a legitimate mixing move and a
score's layers crossfade rather than sum, so any threshold fires on the ordinary case. `needs-decision/above-unity-gain.md`
carries the arithmetic (a layer at +12 dB is 3.98×; `sum_db` does not clip) and the recommendation **not** to add it.

**Verdict: GREAT.**
