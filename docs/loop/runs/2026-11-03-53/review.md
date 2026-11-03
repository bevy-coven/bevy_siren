# review.md — iteration 53, three passes

## Pass 1 — Function: is `Default` the score the file loader gives?

**The claim** (`docs/research/siren.md` R5): *"A change is quantised to the next beat or bar from tempo alone
(`next_boundary`), so no beat map is authored"* — at 120 bpm and four beats, a `NextBar` change from 0.3 s lands at
**2.0 s** and no gain moves before it.

**Measured**, the same score written two ways:

| path | `beats_per_bar` | `bar_s()` | `problems()` |
|---|---|---|---|
| RON, key omitted | **4.0** | `Some(2.0)` | `[]` |
| `MusicScore::default()` | **0.0** | **`None`** | **`["beats_per_bar is not a positive number"]`** |

`MusicScore::default_beats_per_bar()` is **4.0** and serde's field default calls it; `derive(Default)` gave **0.0**. Two
spellings of one field, and the `Default` had the wrong one — so the crate's own default was a score the crate called
broken, and a `NextBar` change had no bar to wait for.

**The fix is one handwritten `Default`.** `a_default_score_quantises_to_the_bar` drives the whole path — a score built
from `Default`, a `MusicLayers::new(..).next_bar()`, `next_change_s()` — and asserts the due time is **2.0 s** with
nothing moved before it. On the old tree it fails at that assertion; on the new one it passes.

**`tempo_bpm: 0.0` was left alone deliberately.** It is documented as *"no beat grid: every transition is immediate, for
a score that is a pad rather than a song"* — zero is a **meaning** there, and forbidden in `beats_per_bar`. One field's
zero means "no grid", the other's means "broken"; only the second needed fixing.

**Verdict: GREAT.**

## Pass 2 — Zoom: what the numbers do at the edges

**`bar_s()` was never wrong.** It returns `None` for `beats_per_bar <= 0.0`, which is the correct answer to zero beats —
not a fast bar, but no grid. `problems()` was never wrong either. Both did exactly the right thing with the number
`Default` handed them. The fault was upstream of both, which is why no amount of reading `bar_s` would find it.

**The tests that would have caught it did not exist.** 26 sites construct a `MusicScore`; the ones that matter set
`beats_per_bar: 4.0` explicitly, and the sites using `..MusicScore::default()` silently took the zero. `tests/music.rs:644`
runs a whole app on `MusicScore::default()` — a score with no grid — and passes, because nothing there asserts a bar.

**A test that would have been wrong.** My first version of the end-to-end test built `MusicLayers { .. }` directly and
called `run_for(&mut app, 0.1)`; the helper takes a different argument list. Rewritten to match
`a_change_waits_for_the_bar`'s shape (`advance`, `.next_bar()`, `next_change_s()`), because a second way of driving the
score would be a second set of rules.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: is the fix visible, and is the audio unaffected?

**No clip, and why.** The bar is not on the panel: `examples/mixer`'s `Music` block draws each layer's `dB -> target`
and a `waiting` marker, and draws neither `next_change_s()` nor `bar_s()`. A capture of this fix would show an unchanged
panel. It names its test instead: `tests/music.rs::a_default_score_quantises_to_the_bar`, which drives the bar and
asserts the 2.0 s due time.

**The audio is untouched, and that is checked rather than assumed.** `bus-Music` still reads `duck -8.0 dB, deepest
-7.96 dB, recovered by 1.97 s`; `mix.wav` still integrates **−18.6 LUFS at −6.1 dBFS**; the music cue's spectrogram
(`evidence/spectrograms/calm-spectrum.png`) is a clean attack at 0, a decay to 0.85 s and silence — a loop-shaped stem
with no click or clipping edge.

**The example was not affected either**, because it sets `beats_per_bar: 4.0` explicitly. The bug lived entirely in code
that trusted `Default`, which is precisely the code that does not check.

**Verdict: GREAT.**
