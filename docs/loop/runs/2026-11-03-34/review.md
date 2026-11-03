# review.md — iteration 34, three passes

## Pass 1 — function

**The claim.** `MusicLayer::cue`'s doc says the cue *"should name the `Music` bus"* — advice, and nothing checked it. A layer
naming a loop on `Effects` started there, so `MusicState` reported two layers live while one was mixed on the effects bus,
`MusicLayers` could not move it, and `MusicScore::problems` said **nothing**.

| row | source | before | after |
|---|---|---|---|
| a layer naming a loop on `Effects` | S1 (`MusicLayer::cue`: *"should name the `Music` bus"*), S2 (stems of **one piece** are mixed together), S3 (the stinger path already refuses this) | **started on `Effects`**; `MusicState::live() = 2`; `problems()` **empty** | **not started**; `live() = 1`; report and log name layer, cue **and bus** |
| the report | S4 (the score's report is `Vec<String>` — a sentence is not an API change) | `[]` | one sentence, word for word the log's |
| a layer naming a `Music` cue (guard) | S1 | starts | **unchanged** |

**Against the sources.** S3 is the sharpest: `play_stingers` already refuses this with a sentence and a comment
explaining that **no `Refusal` variant says it**. The layer path was the same fault with a quieter mechanism — it
*started* the sound rather than refusing it — and the crate's own report did not see it either.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/wrong-bus{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**The measured table**:

```
before                                             after
problems: (nothing)                                one sentence naming layer, cue, bus
voices:   wrong.bus@Effects, track.ok@Music        track.ok@Music
live():   2                                        1
mix:      Effects 1, Music 1                       Effects 0, Music 1
```

Every column moves the same way: the layer is gone from the effects bus, the mirror stops counting it, and the report and
the log print the same words.

**A test defect caught here:** my first version of the test built the good layer with `layer_cue("track.ok")` — a cue whose
sound key is `"track.ok"` — while the sounds list was `["a", "b"]`, so **neither** layer played and the assertion read
`left: []`. The test's own table was wrong; the code was right. Fixed to `["a", "track.ok"]` and the assertion passed.

**The `bad` list now carries its reason.** Iteration 16 built one notice per layer for a non-looping cue; the wrong-bus
case reuses it because the list is `(name, cue, sentence)` rather than `(name, cue)` with the sentence hard-coded at the
print. Two faults, one loop.

**Verdict: GREAT** after the test fix.

## Pass 3 — fresh eyes

**A different shape.** Two tests: the fault (a layer on `Effects` beside a good layer) and the guard (a score of two `Music`
layers still starts both and reports nothing). The fault test asserts on **all three** observable surfaces — the report,
the voices, and the log — because the defect lived in the disagreement between them.

**Can a developer understand it without the code?** `MusicLayer::cue`'s doc still says *"should name the `Music` bus"*; the
report and the log now say what happens when it does not, in the same sentence. A person reading either learns which
layer, which cue, which bus.

**What a fresh reader would still ask:** *why not put the layer on `Music` anyway?* Because the cue's own slot names its
bus, and `Play` has no bus override — moving it would be the crate inventing a routing the table did not author. Refusing
is the honest answer, and it is what the stinger already does.

**Verdict: GREAT.**
