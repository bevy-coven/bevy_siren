# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Entries are **newest first**; iteration 0 is the repository's creation. Rules and traps are in
`AGENTS.md`; the design rules and their sources are in `../research/siren.md`; the fixed cases every iteration runs are
in [`cues.yaml`](cues.yaml); each iteration's own research, plan, evidence and three-pass review are in
[`runs/YYYY-MM-DD-NN/`](runs/README.md), and a question the loop will not decide itself goes to
[`needs-decision/`](needs-decision/README.md).

## Iteration 34 — 2026-11-03 — a layer is a stem of the `Music` bus

**Change.**

- **`src/music.rs` — `start_missing` refuses a layer whose cue's bus is not `Music`**, through iteration 16's one-notice
  machinery. `MusicScore::problems` gains the same sentence word for word, so the report and the log cannot disagree.
- **The `bad` list carries `(name, cue, sentence)`** rather than re-deriving the wording at the print, so both faults share
  one loop and the reason is the thing carried.
- Two tests (one failing on the base commit, one guard); a CHANGELOG line.

**Why.** `MusicLayer::cue`'s doc says the cue *"should name the `Music` bus"* — advice, and nothing checked. A layer naming a
loop on `Effects` **started there**: `MusicState` reported two layers live while one was mixed on the effects bus,
`MusicLayers` could not move it, and `problems()` said nothing. `play_stingers` already refuses exactly this; the layer path
had neither the check nor the line.

**Evidence.** `runs/2026-11-03-34/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/wrong-bus{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `problems()` | **nothing** | one sentence naming layer, cue and **bus** |
| live voices | `wrong.bus@Effects`, `track.ok@Music` | `track.ok@Music` |
| `MusicState::live()` | **2** | **1** |
| `SirenMix` voices | Effects **1**, Music 1 | Effects **0**, Music 1 |
| a score of two `Music` layers (guard) | starts | **unchanged** |
| every test, both feature sets | 116 / 123 / 125 + 5 | 118 / 125 / 127 + 5 |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | median 1.095 µs at 256, load 8.5–5.3; one bus comparison per **voiceless** layer |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A test defect caught in review: my first version built the good
layer with `layer_cue("track.ok")` — whose sound key is `"track.ok"` — against a sounds list of `["a", "b"]`, so **neither**
layer played and the assertion read `left: []`. The test's table was wrong; the code was right.

**Traps for the command.** (1) A doc that says **"should"** is an invariant nobody wrote: grep the `should`s and check each
has a mechanism or a report. (2) When one path already refuses a fault (the stinger), the sibling path that merely *starts*
is the bug — the quieter mechanism is the harder one to see. (3) A notice list that carries `(name, cue)` and re-derives
the sentence at the print cannot grow a second fault; carry the **sentence**. (4) A test's own table is code: a wrong sound
key made a correct fix look broken. (5) `hs distill` GPU-blocked (thirty-third iteration).

**Commit.** `d7b5bb3`, pushed to `main`; CI run 41598441440 is **success** on `big-s1`. Gate: build, test (118 + 5),
serde (125 + 5), mixer (127 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block in the panel** — `MusicState`'s seven accessors drawn by nothing; deferred **three times** for height
   (12 px free against ~134). It needs a layout decision, and every iteration that routes around it should say so.
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-third pass.** The **"should"** sweep is the new lead and it is cheap: `grep -rn 'should\|ought' src/` and check
each sentence has a mechanism. Two found so far this iteration (the layer's bus, now enforced, and `MusicLayer::cue`'s
remaining wording). The next run should finish that sweep across `cue.rs`, `mix.rs` and `play.rs` — a doc that says *"should"*
where the code says nothing is a defect that has shipped in silence.

**Baseline note.** Nothing was red; the suite had 116 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 33 — 2026-11-03 — a queued change follows the score's tempo

**Change.**

- **`src/music.rs` — `Pending` remembers which boundary it was quantised to (`When`) and the tempo it used**, and
  `move_layers` re-quantises a change whose tempo is stale, before the due filter: `at.max(now)` so a faster tempo cannot
  pull it into the past, and a `tempo` guard so an unchanged score does no work.
- Three tests (one failing on the base commit, two guards); a CHANGELOG line.

**Why.** Iteration 32's lead was a Music block, deferred for height. Probing the **music behaviour the block would have
shown** found something larger: `When::NextBar` promises *"on the next bar, quantised with `next_boundary`"*, and
`move_layers` computed that instant **once** from the tempo at request time. A `MusicScore` is a resource *"the Cauldron
edits"*, so a tempo change while a change waits is the ordinary path — and the crossfade landed **off the beat**, in both
directions.

**Evidence.** `runs/2026-11-03-33/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/tempo{,-before}.txt`.

**Measured** — walking the clock until `target_db` moves, so the number is when the crossfade **happens**:

| case | before | after |
|---|---|---|
| 160 bpm, unchanged | 1.44 s | 1.44 s (a bar is 1.5) |
| 80 bpm, unchanged | 2.944 s | 2.944 s (a bar is 3.0) |
| **160 → 80** before the bar | **1.44 s — mid-bar** | **2.944 s — on the bar** |
| **160 → 240** before the bar | **1.44 s — a bar late** | **0.944 s — on the bar** |
| `When::Now` (guard) | own frame | own frame |
| every test, both feature sets | 113 / 120 / 122 + 5 | 116 / 123 / 125 + 5 |
| every rendered cue | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | — | median 0.543 µs at 256, load 5.7–4.5; the loop runs over **queued changes** (0 or 1), not voices |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The **direction of the error was measured both ways** — a
slowdown made the change early, a speed-up made it late — because one direction alone leaves the sign error untested.

**Traps for the command.** (1) An instant **computed once from a resource** is stale the moment that resource changes; a
queue must keep the input it quantised against, not only the answer. (2) Iteration 14 fixed this for **layers** and missed
the **pending queue** in the same module — when fixing a reconcile, list every structure that cached the old value.
(3) A re-quantise needs a guard on *the input's change*, or it moves a queued item forward every frame; the guard test is
what stops that. (4) Measure *when the effect happens* (walk the clock to the observable), not what the accessor reports —
they agreed here, and that agreement is worth knowing. (5) `hs distill` GPU-blocked (thirty-second iteration).

**Commit.** `397b123`, pushed to `main`; CI run 41597804924 is **success** on `big-s1`. Gate: build, test (116 + 5),
serde (123 + 5), mixer (125 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block in the panel** — `MusicState`'s seven accessors drawn by nothing; needs a layout trade (12 px free
   against ~134). Deferred twice now, with the budget measured both times.
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-second pass.** The **music** module has now paid four times (14, 15, 19, 33), each found by probing a
**resource edited at runtime**. What has never been probed there is `MusicScore::problems` against a score that parses and
is wrong in a way the plugin then acts on: a layer naming a cue on a **non-`Music` bus** (`problems` says a layer *"should
name the Music bus"* — does the plugin refuse, or mix it on `Effects` while the score says music?). That is the next probe.

**Baseline note.** Nothing was red; the suite had 113 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 32 — 2026-11-03 — the panel's log shows the eight lines the crate documents

**Change.**

- **`examples/mixer.rs` — `SirenLog::recent(8)`**, the budget `CAPACITY`'s own doc names: *"a panel shows eight"*. The panel
  asked for ten in a fixed column that fits ~8.5, so the **last log line was clipped mid-glyph** in every capture.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` says why eight; a CHANGELOG line.

**Why.** Iteration 31's lead was a Music block. Probing the **height budget** first — iteration 30's lesson is that the
column is fixed — found the panel already over: **lowest ink at `y = 719` of 720**, one pixel of margin and a sliced glyph,
in the README's own still. The crate had documented the budget and the example used a different number.

**Evidence.** `runs/2026-11-03-32/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{eight-log-1280,readme-before}.png`.

**Measured.**

| row | before | after |
|---|---|---|
| lowest ink in the capture | **y = 719** of 720 — 1 px margin, **clipped glyph** | **y = 708** — 12 px margin |
| log lines shown | 10 | **8** — the documented budget |
| free height for the next item | — | **12 px**, against ~134 for a four-line Music block |
| every test, both feature sets | — | unchanged (113 / 120 / 122 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.065 / 0.353 / 1.056 µs, load 3.3–4.6; the panel formats two **fewer** lines |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **Scope was cut deliberately and recorded:** the **Music
block** — the example ships a two-layer score whose `Layer` step really crossfades `calm → combat` on the bar, and
`MusicState`'s five panel-facing accessors are drawn by nothing — needs ~134 px against 12 free. Shipping a clipped log
*and* a block that pushes it off screen would be two defects for one; the budget is in `eval.md` so the next run starts from
a measurement.

**Traps for the command.** (1) A **column that is full clips silently**: `recent(10)` returns ten entries whether the window
shows them, so only a **pixel scan of the capture** can see a sliced glyph. (2) Before adding a section, measure the free
height from the picture; my content arithmetic (~892 px in 720) was right in direction and wrong in magnitude. (3) A crate
that documents a budget (`"a panel shows eight"`) and an example that uses another number is a defect waiting for a capture.
(4) Regenerating an image does not prove it is right — iterations 29–31 each regenerated this one **with** the clip.
(5) `hs distill` GPU-blocked (thirty-first iteration).

**Commit.** `9967090`, pushed to `main`; CI run 41597034221 is **success** on `big-s1`. Gate: build, test (113 + 5),
serde (120 + 5), mixer (122 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block** — `MusicState`'s `layer_db`/`target_db`/`names`/`moving`/`next_change_s` drawn by nothing, and the
   script performs a crossfade the picture cannot show. Needs a layout decision: ~134 px against 12 free, so the cue list
   or another block must give rows. **The budget is measured.**
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-first pass.** The panel class has paid five times (26, 29, 30, 31, 32), and the last four were found by
reading the picture against the code. The **next** defect is already measured rather than guessed: the column has 12 px
free and the music has no view. The run after this one should **choose the rows to give up** — nine cue rows include three
music cues a Music block would show better, which is the natural trade — and prove the result by capture at 1280 **and**
1920, where the extra height means the trade may not be needed at all.

**Baseline note.** Nothing was red; the suite had 113 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 31 — 2026-11-03 — the bus row names the cue that ducked it

**Change.**

- **`examples/mixer.rs` — the bus row says `duck -8.0 dB by dialogue.greeting`** while the duck holds, from the newest
  `Entry::Ducked` naming that bus in the `SirenLog` the panel already holds. The clause is inside the `duck < -0.05` branch,
  so it disappears the frame the duck releases.
- **`tests/mix.rs::the_newest_duck_naming_a_bus_is_the_one_a_panel_shows`** — the rule pinned where the suite can see it:
  a non-duck entry between two ducks does not confuse it, a bus nothing ducked has no name, a newer duck on the same bus
  wins.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` and a CHANGELOG line.

**Why.** Iteration 30's lead asked what the panel could not show. Reading the code answered it: `SirenMix::deepest_duck`
is documented *"for a panel that says what a line of dialogue did"* — and the panel never drew it. Probed further, the bus
row showed **how far** and never **by what**, while `Entry::Ducked` records the cue. A person had to find the log line to
learn which cue lowered the bus, with the answer already in the panel's hand.

**Evidence.** `runs/2026-11-03-31/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{duck-by-1280,duck-by-1920,no-duck-1280}.png`.

**Measured.**

| state | before | after |
|---|---|---|
| Music ducked by a line | `duck -8.0 dB` | `duck -8.0 dB **by dialogue.greeting**`, one line at 1280 and 1920 |
| the duck releases | — | `duck +0.00 -> row shows no duck` (probed) |
| a second duck on the same bus | — | the newest `Ducked` wins (test) |
| a bus nothing ducked | — | no name (test) |
| every test, both feature sets | 112 / 119 / 121 + 5 | 113 / 120 / 122 + 5 |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.066 / 0.332 / 0.884 µs, load 3.9–5.1; the annotation scans the 256-entry log once per **ducked bus** on a debug view |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The **released** state was reached through a probe rather than
a capture, because the scripted run re-fires a duck every 20 frames — drive the state, not the animation.

**Traps for the command.** (1) A method whose doc names a consumer and which that consumer never calls is either a dead API
or a **missing view**; check which before deleting or keeping. (2) A panel row is a **width budget**: measure the annotated
row (71 chars) against the columns that fit (~114 at 1280) *and* look at the capture. (3) An annotation whose condition is
the same as the fact it annotates cannot go stale — render it **inside** the guard. (4) The example is outside the suite, so
pin the **rule** it uses in `tests/`, or a change to `Entry`/`recent` breaks the panel in silence. (5) `hs distill`
GPU-blocked (thirtieth iteration).

**Commit.** `b4f141b`, pushed to `main`; CI run 41596558047 is **success** on `big-s1`. Gate: build, test (113 + 5),
serde (120 + 5), mixer (122 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, twentieth pass.** The panel class has now paid four times (26, 29, 30, 31), and every one was found by reading
the code against the picture. What the panel still does **not** show, and the crate records, is the **music's own state**:
`MusicState::layer_db`, `target_db` and `moving()` are drawn by no view, and the panel's `Cues` block lists `track.calm`
and `track.combat` as ordinary cues when they are **layers of a score**. Next probe: run `examples/mixer` with
`MusicPlugin` and a score, and see whether the panel can tell a layer from a one-shot.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 30 — 2026-11-03 — the panel's `Problems` block is bounded so the log survives

**Change.**

- **`examples/mixer.rs` — the `Problems` block shows three sentences**, and when there are more it prints
  `and N more; the log below is what played`. The panel's fixed column has no scroll and the log is its last section.
- **`SIREN_FAULTS=many`** adds four more deliberate faults, so the case reproduces; `SIREN_FAULTS=1` (two faults, the
  README's GIF) stays under the bound and is unchanged.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` says why the block is bounded; a CHANGELOG line.

**Why.** Iteration 29's lead (`master_db` at silence) probed **clean**, as did *one answer per play* (6 asks → 2 `Played` +
4 `Refused` = 6, each naming the right thing). The probe that paid came from **looking at a capture**: with six faults the
block pushed the `Log` heading and its ten lines off the bottom of a 1280×720 frame — the section that answers *"why is the
music quiet"*, lost exactly when there was most to read. The log block was already bounded at `recent(10)`; the block
above it was not.

**Evidence.** `runs/2026-11-03-30/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{many-faults,many-faults-after,two-faults-after,clean-after,zoom-problems-many}.png`.

**Measured.**

| faults | before | after |
|---|---|---|
| six (`SIREN_FAULTS=many`) | six sentences; **`Log` off screen** | three + `and 3 more; …`; **log on screen** |
| two (the README GIF) | both sentences, log present | **unchanged** — the bound does not fire |
| none | `Problems` / `none` | unchanged |
| every test, both feature sets | — | unchanged (112 / 119 / 121 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| panel frame time | 8.34–8.84 ms | 8.34–8.84 ms; the bound does strictly **less** work on a fault-heavy table |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first version, caught by looking: the count
line used an em dash and the panel's font has no glyph for it — the capture showed `and 3 more ▯ the log`. It is ASCII now.

**Traps for the command.** (1) A **fixed, unscrollable column** is a budget: every section added takes height from the ones
below, and the last one wins or loses. (2) If the panel bounds one list (`recent(10)`) it must bound the next, or the
unbounded one eats the bounded one's space. (3) **A debug font is not a text editor**: an em dash drew a box; keep panel
text ASCII. (4) The failure was invisible in every clean capture — six faults were needed, and `SIREN_FAULTS=many` exists
so the next run can see it. (5) `hs distill` GPU-blocked (twenty-ninth iteration).

**Commit.** `5905ae4`, pushed to `main`; CI run 41596029576 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it (`empty-sound-key.md`,
   `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`, `duck-recovers-entry.md`, the despawn
   record, and now `SIREN_FAULTS=many`'s four faults have no case in `cues.yaml`).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, nineteenth pass.** The panel class has now closed three defects (26's absent block, 29's stale pictures, 30's
overflow), and each was found by **looking**, not by a test. What the panel still cannot show is **`SirenMix::deepest_duck`**
— the bus rows carry a `duck` figure but nothing names *which* duck is deepest or that a duck **finished**; the header
counts cues, buses and voices but not ducks. Next probe: does the panel have any line for a duck that ended, and is the
header's count the one a person needs?

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 29 — 2026-11-03 — the README shows the panel the code draws

**Change.**

- **`docs/media/mixer.png` regenerated** from the clean table — the state a project ships: nine cues, `Problems / none`.
- **`docs/media/mixer.gif` regenerated** with `SIREN_FAULTS=1` — two rows `<- problem`, the block naming both.
- **`docs/media/mixer-panel.gif` deleted** — created by iteration 26, named in no README.
- **Both alt texts rewritten from the pictures**; the GIF's alt describes the animation, not one frame. `README.md` and
  `docs/media/README.md` updated, with the still's and the GIF's regeneration commands.
- **`runs/2026-11-03-29/check_media.py`** — every image a README names exists, and every image in `docs/media` is named
  by one.

**Why.** Iteration 28's lead (a stale `set_voice_db`) probed **clean** — `occupant_mut` compares the packed generation and
the crate's own test already asserts it — as did three more (every `Attenuation` model, a settings swap mid-duck, the
mixer's slot bookkeeping). The probe that paid was the front door: both README images were generated **before** iteration
26's `Problems` block, and the alt text named *"eight cues … eight log lines"* against a shipped table of **nine** cues and
a log of **ten**.

**Evidence.** `runs/2026-11-03-29/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{mixer-png-before.png,mixer-gif-before.gif}`, and `check_media.py`.

**Measured.**

| artefact | before | after |
|---|---|---|
| `mixer.png` | generated **before** the `Problems` block; alt counts wrong | nine cues, `Problems` / `none` |
| `mixer.gif` | same date, same staleness | two rows `<- problem`, block naming both |
| `mixer-panel.gif` | **orphan** | deleted |
| `check_media.py` | — | 6 ok, exit 0; **orphan → exit 1** on a pre-fix reproduction |
| every test, both feature sets | — | unchanged (112 / 119 / 121 + 5) |
| frame cost, 8 / 64 / 256 | — | 0.057 / 0.322 / 1.342 µs, load 3.1–6.6 (the quietest window recorded); no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own alt text, caught by looking: the first
version said *"Dialogue ducking Music by 8 dB"*, but the GIF's first frame shows `Music -2.0 dB` with no duck — the
animation starts at frame 30. Alt text for an animation describes what **happens**.

**Traps for the command.** (1) **Nothing in `cargo test` reads `docs/`**: a stale README picture is invisible to 26
iterations of a loop whose step 6 is "look at the panel". Look at the picture the README shows, not only the one you just
captured. (2) A "regenerate the GIF" step that writes a **new filename** leaves the old one in place and the README
pointing at it — one name per artefact. (3) Alt text must be read off the picture after capture: counts and levels drift
with the panel. (4) **A check that only ever passes proves nothing** — run it against a reproduction of the old state. (5)
`hs distill` GPU-blocked (twenty-eighth iteration).

**Commit.** `a793e10`, pushed to `main`; CI run 41595547980 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **six** items wait on it (`empty-sound-key.md`,
   `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`, `duck-recovers-entry.md`, and the despawn
   record).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, eighteenth pass.** Five probes came back clean this iteration, and the clean list is now long enough to be the
finding: the mixer class, the attenuation models, the settings-swap prunes, the stale-index guard, and the front door.
What has **never** been probed is **`SirenSettings::problems()` against a table it must accept but a host would
misconfigure** — specifically `master_db` at `SILENT_DB` with every bus at 0: does the mix report silence through every
bus, and does `bus_db` say `SILENT_DB` rather than a chain that has one silent stage? Next probe.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Note on the skill's machine facts.** The build-loop text says `1.98.1` is "Bevy's exact floor". The crate's
`rust-version` is **`1.97.1`**, `bevy-0.20.1`'s own `rust-version` is **`1.97.1`**, and `README.md` says MSRV 1.97.1 —
all three agree, so the note is wrong for this dependency. Recorded; the `rust-toolchain.toml` pin at 1.99.0 is what
every `cargo` call here uses and is unaffected.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 28 — 2026-11-03 — a despawned voice frees its mixer slot

**Change.**

- **`src/playback.rs` — `start_pending` keeps the starts it handed to the mixer in a
  `Local<Vec<(Entity, String, usize)>>`** and, once per frame, retains only those still in
  `Query<(Entity, &MixerVoice, &Voice)>`. A start whose entity is gone — its owner despawned — is stopped through the
  trait's existing `stop_voice`, freeing the slot.
- One test behind `mixer`, with its guard in the same test; a CHANGELOG line and a media note.

**Why.** Iteration 27's lead was `Play::on(entity)`. Probed, Bevy's path is fine (its sink dies with the entity); the
crate's mixer **never** called `stop_voice` for a despawned voice, so the `rodio` player sounded for ever with its slot
held — a leak **and** a two-backend divergence the crate's own rule forbids. Invisible on the default path, which is why
27 iterations did not see it.

**Evidence.** `runs/2026-11-03-28/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/mixer-slot{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| owner alive, 10 frames (guard) | `stop_voice []` | `stop_voice []` |
| owner despawned, 10 frames | **`stop_voice []`** — leak | **`stop_voice [0]`** — slot freed |
| `SirenMix::voices_on("Effects")` | 0 | 0 |
| the log after the despawn | `["played engine …"]` | **unchanged — the record gap, named** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.338 µs at 256 | median 0.910 µs, load 10.0–7.4 — a drop this change cannot cause (`perfcheck` has no `Output`, so `start_pending` returns early); reported as measured |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **Two drafts of `research.md`/`plan.md` were wrong**: they
argued the fix needed a new public resource and wrote a decision file instead; probing further showed a `Local` does it
with no public type at all. Both files were rewritten before the code was committed. A component `on_despawn` hook was
considered and rejected — Bevy 0.20 has one, but its `DeferredWorld` cannot reach the `Output` resource.

**Traps for the command.** (1) A component on an entity is **not** a record that outlives it: the crate's own bookkeeping
lived on the voice, so the voice's death erased it. (2) **A leak can hide on the default path** — this one was invisible
without `--features mixer`, which is why the skill's "both feature sets" rule exists. (3) When a fix looks like it needs a
public type, check for a `Local` first: a system parameter is private and often enough. (4) A hook that cannot reach the
resource it must call is not the place; reconcile where the trait object is. (5) `hs distill` GPU-blocked (twenty-seventh
iteration).

**Commit.** `e8a8a32`, pushed to `main`; CI run 41595012489 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
   Needs a new reason, so the public-enum decision.
2. **Answer or route around the `CueProblem` question** — now **six** items wait on it
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`,
   `duck-recovers-entry.md`, and this).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, seventeenth pass.** The mixer class has now paid twice (iteration 17's `AudioPlayer` fallback, and this). Its
remaining untested surface is **`set_voice_db` on a voice whose slot was reused**: the trait takes `(bus, index)` and the
mixer packs a generation, so a stale write should address nothing — case 26 claims that, and `tests/mixer.rs` proves it for
`finished` and `stop_voice`. Whether a **stale `set_voice_db`** is equally harmless is the next probe.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 27 — 2026-11-03 — a duck ends its hold when the voice that justified it stops

**Change.**

- **`src/mix.rs` — `DuckClock::source_went_quiet(live, settings, now)`**, called once per frame in `update_mix` before the
  bus loop. A rule whose `source` bus has **no live voice** has its clock rewound **once**, by the hold that had not yet
  elapsed, so the fade is already underway and keeps advancing.
- **`update_mix` counts live voices before the bus loop**, with the same predicate `SirenMix::voices` uses (`not
  Stopping`), so the duck and the panel agree on what a player hears. `duck_db` keeps its signature and is the single
  read; a vestigial `duck_db_with` was deleted.
- Three tests (one failing on the base commit, two guards); a `buses.md` paragraph; a CHANGELOG line; and
  `needs-decision/duck-recovers-entry.md` for the log entry this does **not** add.

**Why.** `Duck`'s own doc says a duck holds because *"a voice starting on `source`"* is playing; `DuckClock` ran the full
hold whatever happened to the source. A line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and spent
0.6 s recovering — **0.85 s of duck for a sound that had already stopped**. The skill names *"a duck that does not
recover"* as rank-1; this was that failure arriving late.

**Evidence.** `runs/2026-11-03-27/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/duck-hold{,-before}.txt`.

**Measured.**

| case | before | after |
|---|---|---|
| the line is **stopped** on frame 0 | `+0.0` at frame **60** | `+0.0` at frame **40** |
| a **loop** keeps playing (guard) | `+0.0` at 60 | unchanged |
| a **second line** at frame 20 (guard) | extended past 70 | unchanged |
| a **one-shot that ends by itself** (guard) | `+0.0` at 60 | unchanged |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.219 µs at 256 | median 1.338 µs, load 16.9–8.5 — a 2.5× spread within one binary; the change is per **rule**, not per voice |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **One row of four moves**, which is what a correct fix looks
like: only a voice a player can no longer hear releases the hold early, and a one-shot keeps its hold because it is still
live until `retire_voices` despawns it.

**Traps for the command.** (1) An envelope keyed on *when it fired* cannot notice that its justification has gone away;
the clock needs the fact, not just the time. (2) **Clamping** an elapsed time (`max(elapsed, hold)`) pins a fade at the
hold boundary **every frame** so it never advances — the duck held forever, worse than the bug; a **one-time rewind**
moves it forward. (3) Count "what a player hears" with the **same predicate** the panel uses (`not Stopping`), or the
duck and the fader disagree. (4) A fix that leaves a method nothing calls is a vestige; clippy caught `duck_db_with`.
(5) `hs distill` GPU-blocked (twenty-sixth iteration).

**Commit.** `9bf836a`, pushed to `main`; CI run 41594075870 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (120 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it** — now **five** items wait on a public-enum decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`,
   `duck-recovers-entry.md`). Asked in iteration 24; not re-asked since.
2. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. Quantise a **retarget** onto the bar (iteration 14).
5. `Refusal` payload naming the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, sixteenth pass.** The duck class is now closed on the **release** side. Probed the **attack** too, before
writing it down — a second, deeper duck firing while the first is halfway through its fade:

```
after the -8 dB duck recovered halfway:  ducked = -2.80
a -12 dB duck fires:                     ducked = -12.00   (deeper wins; a sum would be -20.00)
```

**Clean**, and exactly what `duck_db`'s doc claims (*"two ducks do not stack: the deeper one wins"*). The duck class is
closed at both ends.

**Next probe:** with the duck closed, the surface left is **`Play::on(entity)`** — a play parented to a game entity. The
crate inserts `ChildOf(target)`, and nothing tests what happens when that entity is **despawned while the sound plays**:
does Bevy despawn the voice with it, does the budget still count it, and does `SirenLog` say anything?

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares
`pub mod audio_panel` twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 26 — 2026-11-03 — the panel shows the crate's verdict on its own table

**Change.**

- **`examples/mixer.rs` — a `Problems` block and a row marker.** `draw_panel` read `SirenMix`, `CueTable`, `SirenSettings`
  and `SirenLog` and **not** `problems()`, so a cue on a bus the mix lacks, or with an inverted pitch range, was drawn
  exactly like a healthy one. The panel now lists `table.problems_in(&settings)` (or `none`) and marks a bad cue's row
  `<- problem`.
- **`SIREN_FAULTS=1`** adds two deliberate mistakes to the capture, so the view can be proved; the shipped table stays
  clean. Documented beside the other knobs.
- `docs/media/mixer-panel.gif` regenerated; a CHANGELOG line.

**Why.** Iteration 25's log recorded the gap, verified. The panel's job is the editor's — make a fault visible — and it
could not distinguish a playable cue from an unplayable one. The crate already computes the sentence
(`CueProblem::Display`) and the Cauldron's panel already chains `problems_in`; the crate's own example did not.

**Evidence.** `runs/2026-11-03-26/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{clean-1280,faults-1280,zoom-problems-clean,zoom-problems-faults}.png`, and `docs/media/mixer-panel.gif`.

**Measured.**

| table | before | after |
|---|---|---|
| clean (11 cues, `Ambience` + `Music`) | no `Problems` section | `Problems` / `none`, no row marked |
| `SIREN_FAULTS=1` | the two bad cues drawn as healthy | both rows `<- problem`; two sentences in the block |
| `problems()` instead of `problems_in` | — | would print **five false `UnknownBus`** on the clean table |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| panel frame time | 8.3–13.6 ms | 8.3–9.7 ms, load 7.4–9.6 — the block is not measurable in the spread |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first version, caught by **looking** at the
1280 capture: the row marker was the full `CueProblem` sentence, and it **wrapped onto a second line**, breaking the cue
column's alignment and repeating the block below. One form per idea — the row says three words, the block says the
sentence.

**Traps for the command.** (1) A panel is a **view of the crate's own verdicts**, not a second formatter: if the crate
computes a sentence, the view should show it rather than a re-description that can drift. (2) A marker that repeats a
sentence already on the same screen is two forms of one idea — and in this case it also **broke the layout**, which only
looking catches. (3) `problems_in(&settings)`, not `problems()`: an example with custom buses gets five false sentences
from the default-four wrapper. (4) A capture that proves a view needs a **faulty** input; keep it behind a knob so the
shipped picture stays clean. (5) `hs distill` GPU-blocked (twenty-fifth iteration).

**Commit.** `4ccd29a`, pushed to `main`; CI run 41593125114 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it.** Four variants wait on one decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`); asked in
   iteration 24, not re-asked since.
2. `Refusal::NotALoop { layer, cue }` — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, fifteenth pass.** The panel now shows the crate's verdict and has been looked at in both states. I wrote,
before checking, that the capture never contains a **released** duck — the script ends on one still holding. Probed
directly rather than inferred from a picture:

```
Music duck per frame after dialogue.greeting, default settings (60 fps)
  frame  0:-8.0  10:-8.0  15:-7.9  20:-6.9  30:-4.7  40:-2.6  50:-0.5  55:+0.0  65:+0.0
```

So the duck **does** recover, by frame 55 of the crate's own frame count — 0.25 s hold plus 0.6 s fade, which is the
crate's documented envelope, and the claim in the previous draft of this entry was **wrong**. What the *capture* shows
depends on the script the capture runs, not on the crate: `steal,layer,duck` cycles every `SIREN_KEY_EVERY` = 20 frames
from frame 30, so a `duck` step fires often enough that a released duck is on screen for part of the run.

**Next probe:** the panel's `duck` column shows the *current* duck only. Nothing on screen says a duck **finished** — the
log's `ducked` lines have no matching `recovered` line, so a reader sees the dip and not the release. Whether that is
worth a line is the next question, and it is a `SirenLog` entry, so it is **public API** and goes to a decision file
before it is built.

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 25 — 2026-11-03 — the capture harness runs with no environment at all

**Change.**

- **`examples/mixer.rs` — `capture_config` gives `SIREN_KEYS` the default its doc describes.** Unset used to yield an
  empty step list, and `stage_keys` evaluated `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release
  --example mixer` **panicked and wrote no still**. The default is now `steal,layer,duck`; a `SIREN_KEYS` that parses to
  nothing prints one line naming the typo before using it.
- **The panel, captured and looked at for the first time** at 1280×720 and 1920×1080, with a bus/log crop and
  `docs/media/mixer-panel.gif` (18 frames via `SIREN_SHOT_DIR`); a `docs/media/README.md` entry with the commands.
- A CHANGELOG line and a media note.

**Why.** Iteration 24's lead was `examples/mixer`, and running it was the probe. `%` on an integer panics in every
profile, so this was not a debug assertion: the crate's only **visible** harness died on the command its own module doc
gives, which is why every visible change this loop has shipped was evidenced by the headless `examples/render` alone.

**Evidence.** `runs/2026-11-03-25/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{mixer-1280,mixer-1920,zoom-buses,zoom-log}.png`, `evidence/panel/default-run{,-before}.txt`, and
`docs/media/mixer-panel.gif`.

**Measured.**

| command | before | after |
|---|---|---|
| `SIREN_FRAMES` only | **panic**, no still | still written at both sizes |
| `SIREN_KEYS=bogus` | panic | one line naming the typo, then the default |
| `SIREN_KEYS=layer` (guard) | works | unchanged |
| no environment at all | panic | 20 s, no panic |
| panel frame time (new figure) | — | 8.3–13.6 ms mean, an example's number, not the crate's |
| crate frame cost, 8 / 64 / 256 | 0.061 / 0.348 / 1.244 µs | unchanged — no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The panel **passes the skill's own pass-3 test**: every
number on a bus row (`dialogue.greeting` ducked Music by −8.0 and Effects by −4.0; `engine.idle` `stolen by sword.hit`)
is one the log beneath it explains, so a developer reads it without the code. One crop mistake of mine is recorded rather
than quietly fixed: `zoom-buses.png` starts at `Ambience`, because I offset by 150 px and the Music row *is* at 150.

**Traps for the command.** (1) A documented **optional** knob that is load-bearing is a crash waiting for the default
command. (2) `% len()` on a list built from an env var needs the empty case answered before the modulo, not after.
(3) An example that cannot run cannot be **looked at**, and this loop's step 6 depends on looking — the missing panel
evidence was not laziness, it was a panic. (4) When a crop looks wrong, check the offset before the panel: `zoom-buses`
starts at `Ambience` and the full still has all four buses. (5) `hs distill` GPU-blocked (twenty-fourth iteration).

**Commit.** `97f79ff`, pushed to `main`; CI run 41592625502 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it.** Four variants wait on one decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`); iteration 24
   asked, and this one did not re-ask. If the owner does not want the break, the next run should pick outside the enum.
2. `Refusal::NotALoop { layer, cue }` — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, fourteenth pass.** The visible-harness class is now open in a way it was not: the panel exists as a picture
and **has never been checked against a table the crate rejects**. Probed before writing it down — `draw_panel`'s four
resources are `SirenMix`, `CueTable`, `SirenSettings` and `SirenLog`, in that order, and **`problems()` is not among
them**:

```rust
fn draw_panel(mix: Res<SirenMix>, table: Res<CueTable>, settings: Res<SirenSettings>, log: Res<SirenLog>, …)
```

So the panel draws a cue on an unknown bus, or one with an inverted pitch range, exactly as it draws a good one. That is
the editor's whole job — making a fault visible — and it is the **next item**, measured rather than guessed.

**The hunt, fifteenth step (recorded, not built):** the Cauldron's `audio_panel.rs` *does* chain `problems_in(&read)` into
its status line, so the editor has the line the crate's own example lacks. The gap is in the example, and it is named
above so the next run does not have to find it again.

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 24 — 2026-11-03 — a refusal outlives a burst of plays

**Change.**

- **`src/log.rs` — `SirenLog::push` drops the oldest *routine* entry when the buffer is full**, not simply the oldest.
  `Played`, `Stopped` and `Ducked` happen at frame rate; a healthy game produces no `Refused` at all. If every entry is
  a refusal the oldest still goes. The buffer stays `CAPACITY` (256) — same memory, same `recent`/`iter`/`len`.
- Four tests, two failing on the base commit and two guards; a CHANGELOG line and a media note.

**Why.** Iteration 23's lead was the log's cap under a burst. Its **arithmetic** was clean (capped, oldest dropped,
`recent(n)` safe past the buffer, no growth), and so were four other probes. The one that paid asked a different
question: **how long a *refusal* lives**. One `Refused` was evicted by exactly **256 plays** — **0.40 s** at a 32-voice
scene writing 640 entries a second. That is the silent-failure class the log exists to prevent.

**Evidence.** `runs/2026-11-03-24/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/refusal-lifetime{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| one refusal, then plays | evicted after **256 plays** | survives **2000**; leaves at 256 *routine* entries |
| a 32-voice scene at 640 entries/s | **0.40 s** | ≥ 0.40 s, and longer in a quiet scene |
| 600 refusals (guard) | len 256, oldest `c344` | unchanged |
| 1000 mixed pushes (guard) | len ≤ 256 | unchanged |
| cost, 200k routine pushes | — | 11.1 ms (**55 ns** each) |
| cost, 1000 pushes into an all-refusal log | — | 272 µs (**272 ns** each, pathological) |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.391 µs at 256 | median 1.267 µs, load 8.8–13.8 — within the spread; the log is not on `perfcheck`'s path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction in my own evidence, recorded: the first probe
printed "3.12 s" for the after case by falling back to its 2000-play limit — a made-up number for a rule that has no
fixed play count. It now states the rule (256 *routine* entries) and the floor that follows.

**Traps for the command.** (1) A **ring buffer** with one eviction rule treats a routine fact and an exceptional one
alike; the exceptional one is what a person reads the buffer for. (2) Measure the *lifetime* of the entry a diagnostic
depends on, not just the buffer's size and accessors — the arithmetic was right and the retention was wrong.
(3) A probe that reports "survives N" by hitting its own limit has measured nothing; state the rule, not the loop bound.
(4) Five probes came back clean and are written down: the cap, `recent(n)`, `line()` (host-only), `next_boundary` at
awkward tempos, and every no-ears spatial shape. A clean probe is a result. (5) `hs distill` GPU-blocked (twenty-third
iteration).

**Commit.** `200a119`, pushed to `main`; CI run 41591975363 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **`CueProblem` for an empty `sound` key** — `needs-decision/empty-sound-key.md`; it plays and is not reported.
2. **`CueProblem` for a duplicate key** — `needs-decision/duplicate-sound-problem.md` (iteration 22).
3. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
4. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
5. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.

**The hunt, thirteenth pass.** The log class is closed. Four `CueProblem` variants now wait on one public-enum decision,
and the loop should stop re-finding them — the next run should **ask the owner** whether to take the break, or pick
something outside that enum.

Probed `SirenSounds::failed` first, before writing it down:

```
load::<AudioSource>("audio/does_not_exist.wav")
  frame 0: Some(Loading)
  frame 1: Some(Failed(MissingAssetLoader { asset_path: "audio/does_not_exist.wav" }))
  failed(table) = ["missing"]                 <- correct, and it names the key
```

So the **Failed path works** — my first probe had simply not advanced the app, which is this loop's own recurring trap
(a `Loading` state read on frame 0 is not a verdict).

One state is named by neither accessor: a **`Handle::default()`** inserted for a key is *mapped* (so `missing()` skips it)
and never loads (so `failed()` skips it). It is reachable only if a host inserts a placeholder handle, which is not a
project's table doing anything wrong — **noted, not an item.**

**Next probe:** with the log and `SirenSounds` classes closed, the surface left is **`examples/mixer`** — the one visible
harness the loop has never inspected frame by frame, and the item the skill asks about when a change is in the crate:
does it render every bus the table has, and does a bus it does not list appear as zero?

**Baseline note.** Nothing was red; the suite had 105 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 23 — 2026-11-03 — the seed the stream was built from

**Change.**

- **`src/model.rs` — `Rng::ZERO_SEED` and `Rng::seed()`.** `Rng` was a bare `u64` holding the live xorshift state; it now
  also remembers the seed it was **built from**, so `seed()` answers a number that does not move as the stream draws.
- **`src/play.rs` — `SirenChoices::new` records `rng.seed()`**, so `SirenChoices::seed()` — *"the seed this stream was
  built from"* — is true by construction.
- Four tests; a paragraph in `docs/guide/cues.md`; a CHANGELOG line; and `needs-decision/empty-sound-key.md` for a
  separate gap found while probing.

**Why.** `Rng::new(0)` maps a zero state to `0x5EED_51AE_0000_0001`, which is **also**
`SirenSettings::default_seed()`. So `seed: 0` and the default are **one stream**, and `SirenChoices::seed()` reported the
argument — `0` — for a stream that started somewhere else. Two identical streams wearing two different numbers, and the
one that says `0` is a lie. The reproducibility promise is the crate's own stated reason for a seeded stream (*"a sound
that cannot be reproduced cannot be debugged"*), so the report has to name the seed the sound came from.

**Evidence.** `runs/2026-11-03-23/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/seed{,-before}.txt`.

**Measured.**

| settings | before: `seed()` | after |
|---|---|---|
| `seed: 0` | **`0`** — a number the stream did not come from | `6840213216811810817` |
| default seed (guard) | `6840213216811810817` | `6840213216811810817` |
| `seed: 1` (guard) | `1` | `1` |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.31 µs at 256 | median 1.391 µs, load 10.2–17.5 — within the spread; the change is one `u64` written once |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first attempt is recorded: `Rng::seed()`
initially read the **live** xorshift state, which changes on every draw — the test
`a_streams_seed_does_not_move_as_it_draws` fails against that version and is why it exists.

**Traps for the command.** (1) A **sentinel** that collides with a real value is a lie waiting to be told: `0` mapping to
the default seed's constant means two names for one stream, and the report must name the value the stream holds.
(2) A method documented as *"the seed this stream was built from"* must read something that **does not move** — the live
state is the wrong field. (3) A lead that comes back **clean** is a result: the `Rng` stream is uniform, long, and never
returns 1.0, so the index clamp is defensive — written down so the next iteration does not re-probe it. (4) Check a new
name against **every** other name check: bus names and cue names are trimmed-checked, a slot's `sound` key is not.
(5) `hs distill` GPU-blocked (twenty-second iteration).

**Commit.** `7aff968`, pushed to `main`; CI run 41591211228 is **success** on `big-s1`. Gate: build, test (105 + 5),
serde (112 + 5), mixer (113 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **`CueProblem` for an empty `sound` key** — `needs-decision/empty-sound-key.md`; it plays and is not reported.
2. **`CueProblem` for a duplicate key** — `needs-decision/duplicate-sound-problem.md` (iteration 22).
3. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
4. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
5. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.

**The hunt, twelfth pass.** The **seed** class is now closed: the mapping is named, reported, and pinned as behaviour.
Four `CueProblem` variants are waiting on one public-enum decision, and the loop should stop re-finding them.

Probed the **cue table as a replaced resource** — `CueTable` is a `Resource`, and iterations 13/14 fixed `SirenSettings`
and `MusicScore` replacements, so this was the last one:

```
table swapped for one with no `old`, then StopCue { cue: "old" }
  40 frames later:  []       no voice, no `Stopping`
  no swap, same test:  []    identical
  log: stopped `old`: cue stopped   played `old` (a) on Effects at 0.0 dB
```

**Clean.** A voice outlives the table that named it, fades, and despawns exactly as it does without the swap — which is
right: the voice's own `Voice` record carries what it needs, and nothing reads the table on the way out. My first probe
reported `live 1` only because it counted `&Voice` without excluding `Stopping`, which is the same mistake this loop has
caught before; the corrected probe is the one above.

**Next probe:** the **`SirenLog` cap under a burst** — `CAPACITY` is 256 and `push` evicts the oldest, but nothing asserts
that the entry a panel is *currently drawing* survives a frame that writes 300 entries, or that `recent(n)` with `n`
larger than the buffer returns what it has rather than panicking.

**Baseline note.** Nothing was red; the suite had 101 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 22 — 2026-11-03 — `AvoidRepeat` avoids the sound, not the slot index

**Change.**

- **`src/model.rs` — `VariantPicker::pick_from(&[&str], avoid, rng)`.** The avoided window is made of **keys**, so two
  slots naming one file are one candidate for repetition. `pick(len, ..)` is unchanged for every caller with no keys,
  and the two agree when keys are distinct (a test seeds both from `Rng::new(7)`).
- **`src/play.rs` — `SirenChoices::pick` calls it** with the cue's slot keys, so the play path gets the fix with no API
  change.
- Four tests; a *"the window is made of sounds"* paragraph in `docs/guide/cues.md`; a CHANGELOG line; and
  `needs-decision/duplicate-sound-problem.md` for the report half.

**Why.** Iteration 21's lead was colliding slot keys. One key → two files is not representable (by design), but the
**same key twice in one cue** is, and the picker could not see it: it refused **indices**, so under the crate's *default*
`Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording **three times running**
(`… b a a a b`), and `["a","a","a","b"]` played four. That is the machine-gun footstep `Repeat` exists to kill — the
crate's headline feature — defeated by a copy-pasted line, on by default, with `problems()` silent.

**Evidence.** `runs/2026-11-03-22/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/duplicate-keys{,-before}.txt`.

**Measured.**

| cue, `AvoidRepeat { avoid: 1 }`, 16 plays | before | after |
|---|---|---|
| `["a", "a", "b"]` | **3** adjacent repeats | **0** |
| `["a", "a", "a", "b"]` | **8** | **0** |
| `["a", "b"]` (control) | 0 | 0 |
| `["a"]` (one sound, guard) | 15 | 15 |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.31 µs at 256 | same binary spread 0.393–1.350 µs across five runs, load 8.6–16.5; the change is once per **play** |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction recorded rather than smoothed: the first
perf run read 2.5× *faster* than iteration 21 — impossible for a change that adds work — so five runs were taken and
the spread reported instead of the flattering sample.

**Traps for the command.** (1) A guarantee about what a **player hears** must key on what they hear: an index is the
crate's internal name for a slot, and two slots can name one recording. (2) When a public method gains a keyed variant,
keep the old one byte-identical and **prove** the two agree on the inputs where they should — that is the no-regression
half. (3) A perf figure from one run on a machine at load 16 is a scheduler measurement; take five and report the spread.
(4) A test file behind a feature gate can run zero tests under a filter that misses — count the result, do not read "ok".
(5) `hs distill` GPU-blocked (twenty-first iteration).

**Commit.** `51e9bf3`, pushed to `main`; CI run 41588637059 is **success** on `big-s1`. Gate: build, test (101 + 5),
serde (108 + 5), mixer (109 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (blocker above, re-confirmed this iteration: the same two compile errors).

**Next 5.**

1. **`duplicate-sound-problem.md`** — a `CueProblem` for a duplicate key, conditional on `AvoidRepeat` (option B).
2. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
3. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. `load_table`'s path join — a key of `../outside` becomes `audio/../outside`; named here, not built.

**Consumer status.** `bevy_siren` is green. The Cauldron's audio panel test **cannot** pass: `bevy_grimoire`'s `main`
is still `e00d1f1`, still with two `pub mod audio_panel;` and no `AudioPanelPlugin` — iteration 21's blocker, unchanged,
not mine. My consumer commit `799fad2` sits on local `main`, ahead by one, waiting.

**The hunt, eleventh pass.** `SirenSounds` is now closed: one key → one handle is by design, a duplicate in one cue is
fixed here, and `load_table`'s path join is named. Probing the **`Rng` stream** next, before writing it down:

```
seed u64::MAX, 2,000,000 draws:  max next_f32 = 0.999999583,  values >= 1.0: 0
```

So `next_f32` never reaches `1.0`, and the `min(choices.len() - 1)` clamp in `pick` is defensive rather than load-bearing
— **clean**, no item. What remains genuinely unprobed is **`Rng`'s period**: a xorshift64\* with seed `0` still produces a
stream (measured: `0.7037902, 0.34525365, 0.2853828`), so the seed is mixed rather than used raw, but nothing asserts how
many draws it takes to cycle. That is a determinism question, not a sound one, and it is the next probe.

**Baseline note.** Nothing was red; the suite had 97 tests. `cues.yaml` case 31 is red by design.

---

## Iteration 21 — 2026-11-02 — `problems_in`, and the report's blind spot recorded

**Change.**

- **`src/cue.rs` — `CueTable::problems_in(&SirenSettings)`**, the report against **the buses the mix actually has**. It
  asks `settings.bus_names()`, the one answer `answer`'s `UnknownBus` check, `update_mix` and `SirenSettings::problems()`
  all read. `problems()` keeps its documented default-four meaning, and its doc now says what it assumes.
- **`bevy_grimoire` (its own commit)** — the Cauldron's audio panel calls `problems_in(&read)`; it was calling
  `table_read.problems()` while its own bus list, two lines below, was built from the settings.
- `cues.yaml` gains **case 31, `state: red`**, `blocked_on:` the gain decision; `docs/guide/cues.md` stops implying the
  check list is complete; `bad-gain-problem.md` gains the evidence; CHANGELOG and a media note.

**Why.** Iteration 20's lead (`serde`) came back **clean** — `to_ron` writes `NaN`, `from_ron` returns `NaN`, an unknown
field is refused by name. The probe found the report's five readers, of which four ask `bus_names()` and the fifth — the
one **the editor calls** — assumed the defaults. In a project with a `Sfx` bus, `problems()` printed
*"cue \`gunshot\` plays on bus \`Sfx\`, which the mix table does not have"* for a cue the crate **accepts and mixes**.

**Evidence.** `runs/2026-11-02-21/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/{custom-buses,gain-report}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a cue on a custom `Sfx` bus | `problems()` → **false `UnknownBus`**; the panel drew it | `problems_in(&settings)` → **empty**; the panel calls it |
| the same table with no buses listed | both agree — the crate's four | **unchanged** (the guard) |
| `Cue::gain_db: NaN` / slot `NaN` | 0 sentences | **unchanged — blocked** (case 31, red) |
| `Cue::pitch` inverted / `attenuation` negative | 1 sentence each | unchanged — the asymmetry the red case records |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.046 / 0.327 / 0.991 µs | 0.055 / 0.334 / 1.021 µs, load 7.9–16.3 — within noise; `problems_in` is a **host** call |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction in my own work: the new test first went into
`tests/table.rs`, which is behind `#[cfg(feature = "serde")]`, so the filter ran **zero** tests and reported success.
Moved to `tests/mix.rs`, it now fails to **compile** on the unfixed tree — the strongest "fails before".

**Traps for the command.** (1) A **convenience wrapper** that assumes a default is a trap the moment the crate supports
the non-default: name it for its assumption (`problems_with`), not for the general verb (`problems`). (2) A report is not
fixed when the *function* is right — check **every caller**, and an editor is a caller. (3) A test file behind a feature
gate silently runs zero tests under a filter that misses: confirm the count, not the word "ok". (4) A lead that comes back
**clean** is a result, not a wasted pass — write down what was probed so the next iteration does not repeat it.
(5) `hs distill` GPU-blocked (twentieth iteration).

**Commit.** `82d227e` (library; CI run 41585200646 is **success** on `big-s1`). **The consumer fix is committed in `bevy_grimoire` as `799fad2` but NOT
pushed**, because `origin/main` there does not compile with `bevy_siren` — proved from a clean worktree of `e00d1f1`:

```
error[E0428]: the name `audio_panel` is defined multiple times   (lib.rs:53 and lib.rs:109)
error[E0425]: cannot find value `AudioPanelPlugin` in module `crate::audio_panel`
```

The root cause, traced with `git log -S`: commit **`ca287d7`** added `app.add_plugins(crate::audio_panel::AudioPanelPlugin)`
to `editor.rs` and **never wrote the plugin** — `git log -S AudioPanelPlugin` finds that call and nothing else, at any
commit, in `audio_panel.rs` or in `lib.rs`. Merge **`e7ba838`** then added a **second** `pub mod audio_panel;` to
`lib.rs`. So `main` has been unable to build with `bevy_siren` since that commit, and it is not a merge artifact: it is
an unfinished feature commit. Proven from a clean `git worktree` of `e00d1f1`, twice, with the same two errors.

I did not repair it. It is someone else's commit on a shared `main`, the loop's rules have a consumer breakage fixed as
its own commit only when it is **my** change's breakage, and the two edits (a missing plugin and a duplicated module) are
a design decision about who owns the panel plugin, not a typo I can settle. The library half is shipped and green; the
consumer half waits on that fix. `799fad2` sits on local `main`, ahead by one, rebase-clean, and will fast-forward the
moment `main` compiles again. **Reported, not worked around.**

**Next 5.**

1. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
2. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, tenth pass.** The **report** class is now closed on every reader: the cue table (this iteration), the mix
table (20), the score (19), and the log (16). What has never been probed is **`SirenSounds`** — the key → handle map a
game loads — against a table whose slot keys collide (`SoundSlot::sound` naming the same file twice in one cue, or the
same key in two cues with different files). That is the next probe.

**Baseline note.** Nothing was red; the suite had 96 tests.

---

## Iteration 20 — 2026-11-02 — the report agrees with the mix about buses

**Change.**

- **`src/mix.rs` — `SirenSettings::problems()` asks `bus_names()`, and names each fault once.** It built its own bus set
  from `self.buses`, so a table with `buses: []` — documented as *"the crate's four … everywhere"* — was reported as
  having **four false missing buses** while the crate mixed it correctly. And a duck fault was pushed once per rule, so
  the crate's own default pair (`Dialogue → Music`, `Dialogue → Effects`) printed the same sentence twice.
- `tests/mix.rs` — four tests, three failing on the base commit, the fourth the default-table guard; a `buses.md`
  paragraph; a CHANGELOG line and a media note.

**Why.** Iteration 19's log ended with a **probe, not a guess**, and it found two faults in one function. The worse one is
the false report: a table the crate **mixes correctly** was reported as four mistakes, which is worse than no report —
it teaches an author to ignore the panel. This is the last caller bypassing `bus_names()` (iteration 10 closed the same
bypass in `update_mix`).

**Evidence.** `runs/2026-11-02-20/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/report{,-before}.txt`.

**Measured.**

| table | before | after |
|---|---|---|
| `buses: []` (the crate's four) | **4** sentences, all false | **0** |
| two buses (a menu mix) | **2** identical for `Dialogue` | **1** |
| one bus + two rules naming `Ghost` | **2** identical | **1**, exact words |
| default table (guard) | 0 | 0 |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.064 / 0.361 / 1.311 µs | 0.046 / 0.327 / 0.991 µs, load 8.12/14.44/19.28 — every row faster, which is the load and not this change; `problems()` is a **host** call, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One defect in my own fix, caught by the **pre-existing**
test: deduplicating by **bus name** collapsed `Nope → Music` and `Music → Nope` — two genuinely different faults about one
absent bus — into one sentence. The key must be the **sentence**: identical sentences are one fault, different sentences
about one bus are two.

**Traps for the command.** (1) A report that **invents** faults is worse than one that misses them: it trains the reader
to ignore it. (2) When a resource has a `bus_names()`-style accessor, every reader must use it — a second set is a second
rule, and the crate's own trap list is about exactly that. (3) Deduplicate a report by the **sentence**, not by its
subject: two rules about one bus can be two faults, and one rule pair about one bus can be one. (4) A **pre-existing**
test is the cheapest specification you have — the old `problems_names_each_mistake` caught the over-merge before it
shipped. (5) `hs distill` GPU-blocked (nineteenth iteration).

**Commit.** `8a41eae`, pushed to `main`; CI run 41584532000 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (96 + 4), serde (103 + 4), mixer (104 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (iteration 18).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, ninth pass.** The **report** class has now paid four times (16, 18, 19, 20) and is closed: `problems()` on
the settings and the score, the log, and the refusals all say what the crate does. Probing the budget next produced
something I could not honestly call a bug, and it is written down here rather than dropped:

**`cues.yaml` case 8 is internally inconsistent.** It authors `engine.idle` with `bus_max_voices: 2`, `mode: Loop` and
**no `max_instances`** — so the cue keeps its default `max_instances: 1` — then expects *"Two voices live, one steal
logged, and no third `Played`"* from three requests in one frame. Measured:

```
three requests, one frame, bus cap 2, max_instances 1     played 3, refused 0, live voices 1
```

The crate is **self-consistent**: each request steals the one previous voice of its own cue (the cue cap, not the bus
cap), so the bus never exceeds 2 and one voice is live — exactly what `max_instances: 1` means. A test that wants two
live voices must author `max_instances: 2`, which `tests/mix.rs::a_full_bus_steals_the_oldest_and_then_refuses` already
does with `cue("hit", "Effects", 0, 4)`. I tried making an unlimited cue respect the bus cap as a fix; the crate's own
test showed that would break a documented behaviour, so it was **reverted** and `src/play.rs` is untouched. The case's
`authored` block should gain `max_instances: 2` — a `cues.yaml` edit, which the loop's rules reserve for a new behaviour,
so it is recorded for the owner rather than made.

**Next probe:** nothing in the budget. The untested surface is the **`serde` round trip against a table that parses and
is wrong in a field no `problems()` reads** — starting with whether `from_ron` accepts a `max_instances` past the
table's `max_voices`, and whether `to_ron(from_ron(x))` is stable for a table with a non-finite number in it.

**Baseline note.** Nothing was red; the suite had 92 tests.

---

## Iteration 19 — 2026-11-02 — a music layer's gain is a caller value like any other

**Change.**

- **`src/music.rs` — `start_layers` normalises a layer's starting `gain_db` and `move_layers` every `MusicLayers`
  target** with iteration 18's `model::finite_or`, so a gain that is not a number is **no gain** (`0.0` dB), not `NaN`
  written into `db`, `target_db` and the voice's `base_db`.
- **`MusicScore::problems` gains one sentence**: `"music layer \`calm\` has a gain that is not a number"`. The score's
  report is `Vec<String>`, unlike `CueTable`'s `Vec<CueProblem>`, so **no public enum changed** — this item is complete
  with no decision file, which is why it outranked the waiting `Refusal` items.
- `tests/music.rs` — four tests, three failing on the base commit, the fourth the `SILENT_DB` guard; a guide section; a
  CHANGELOG line and a media note.

**Why.** Iteration 18 ended with a **measured** lead, and it was real: iteration 18's rule covered the five numbers that
enter `answer` and missed the one that enters `start_layers`. The score's report *already spoke* about a non-finite
number — it named a bad `tempo_bpm` — and said nothing about a bad gain.

**Evidence.** `runs/2026-11-02-19/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/music-gain{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a score layer at `gain_db: NaN` | `layer_db NaN`, `base_db NaN`, sink at **−80 dB**, `problems()` **empty** | `0.0`, `0.0`, `Applied 0.0`, one sentence |
| a `MusicLayers` move to `NaN` | `layer_db NaN`, `target NaN` | `0.0`, `0.0` |
| a layer started at `SILENT_DB` (guard) | −80 dB | **unchanged** — `SILENT_DB` is finite |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.079 / 0.357 / 1.288 µs | 0.064 / 0.361 / 1.311 µs, load 8.07/9.79/13.07 — within noise; the change is once per layer, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two corrections are recorded rather than hidden: the
severity is **silent until moved**, not permanent (`approach(NaN, x)` returns `x`), and the move test first failed
because `MusicLayers::new` defaults to `When::NextBar` and the test ran 0.5 s.

**Traps for the command.** (1) A rule applied to *some* doors is worse than no rule: it makes the crate look consistent
while one field still lies. When adding a rule, enumerate **every** entry point. (2) Check what a report's type is before
designing the fix: `MusicScore::problems` is `Vec<String>` and `CueTable::problems` is `Vec<CueProblem>`, and that
difference is whether a fix needs a decision file. (3) A **finite** sentinel is not a bad number — `SILENT_DB` must
survive any normalisation, and that is a test, not a comment. (4) Before claiming "permanent", probe recovery: the
crate's `approach` already guards a non-finite current value. (5) `hs distill` GPU-blocked (eighteenth iteration).

**Commit.** `17eb4a5`, pushed to `main`; CI run 41579423077 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (92 + 4), serde (99 + 4), mixer (100 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (iteration 18).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, eighth pass.** The non-finite class is now closed at every door a **gain or detune** has: `answer`'s five
numbers (18), a layer's start and a `MusicLayers` target (19). Probing the bus level **before** writing it down, because
the obvious guess was wrong:

```
BusLevel::new("Music").gain_db(NaN)
  SirenSettings::problems()  ->  ["bus `Music` has a gain that is not a number"]   <- reported
  SirenMix::bus_db("Music")  ->  -80.00                                            <- and silent
```

So a bad bus level is **reported and silenced**, which is the opposite of this iteration's case (silent and
unreported). Whether that is right is a judgement, not a defect: a bus is the mix's own protection, and `chain_db`'s
silence is what keeps a `NaN` from making every voice on it audible. **No item here** — and the probe also showed
`problems()` reporting the default duck's `Dialogue` source twice when the table lists two buses, which is a
**duplicate sentence**, and that *is* a small defect worth its own item.

**Baseline note.** Nothing was red; the suite had 88 tests.

---

## Iteration 18 — 2026-11-02 — a number that is not a number

**Change.**

- **`src/model.rs` — `finite_or`, `ratio_to_semitones`, `played_semitones`.** One rule for every non-finite caller value
  the crate accepts: it is the **neutral** value for what it measures, which is what `distance_gain` already does for a
  distance and `docs/guide/spatial.md` already states (*"a `NaN` transform gets a sound at the listener's feet, not a
  `NaN` volume that silences the whole bus"*).
- **`src/play.rs` — `answer` normalises the four numbers that enter from a caller** (a cue's gain, a slot's gain, a
  `Play`'s gain override, a `Play`'s detune) and reports the detune the device is **actually at**.
- `tests/play.rs` — four tests, three failing on the base commit; a *"A number that is not a number"* section in
  `docs/guide/buses.md`; a CHANGELOG line; and `needs-decision/bad-gain-problem.md` for the two stricter answers.

**Why.** Iteration 17's lead (the `AudioOutput` trait's contract) came back **clean**: `Applied` records what the crate
decided, and a backend that ignores it desynchronises nothing the crate reads. The probe that paid was different: the
crate had **two opposite answers to one caller mistake**. A `NaN` distance was the neutral `0.0` m; a `NaN` gain was
silence. A cue authored `gain_db: NaN` — which a RON file can contain — **played silent** while `Played` reported
`-80.00` as though that level were authored, and `problems()` said nothing. Worse, `Played.pitch` reported `NaN` for a
sound `semitones_to_ratio` was already playing at `1.0×`: the meter disagreed with the device, which is the one thing
R6's second half forbids.

**Evidence.** `runs/2026-11-02-18/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/nan{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a cue authored `gain_db: NaN` | `gain -80.00`, **silent**, no problem reported | `gain 0.00`, audible |
| `Play::gain(NaN)` | `gain -80.00` | `gain 0.00` |
| `Play::gain(-∞)` | `gain -80.00` | `gain 0.00` |
| `Play::pitch(NaN)` | `pitch NaN` for a `1.0×` sound | `pitch 0.00` |
| `Play::pitch(1e9)` | `pitch 1000000000.00` for a `16×` sound | `pitch 48.00` |
| a bus at `SILENT_DB` (guard) | silent | **unchanged** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.077 / 0.370 / 0.713 µs | 0.079 / 0.357 / 1.288 µs, load 8.72/4.85/4.83 — the 256 row sits in a loaded first sample; the change is once per **play**, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The judgement this iteration had to make — neutralise or
refuse — is written down in `needs-decision/bad-gain-problem.md` with what each costs a consumer.

**Traps for the command.** (1) Two answers to one caller mistake is a bug even when both answers are individually
defensible: find the **stated policy** and make the other path match it. (2) A **meter** that reports the number asked
for rather than the rate that plays is wrong even when the sound is right — `semitones_to_ratio` had the audio correct
and `Started.pitch` re-broke it. (3) `chain_db`'s silence rule is right for a **bus level the crate computed** and wrong
for an **input**; the boundary is "did the crate compute this number". (4) Normalising a bad input is a **behaviour
change** and belongs in a decision file, not in a patch's fine print. (5) `hs distill` GPU-blocked (seventeenth
iteration).

**Commit.** `fdbc1d5`, pushed to `main`; CI run 41578642257 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (88 + 4), serde (95 + 4), mixer (96 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (this iteration).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, seventh pass.** Two structural leads have now come back **clean** (the two-backend answers, the
`AudioOutput` contract). Probing the third before writing it down: **a `MusicLayer` whose `gain_db` is `NaN`** does
reach `MusicState` unfiltered — measured,

```
MusicLayer { name: "calm", cue: "t", gain_db: NaN }
  layer_db = Some(NaN), target_db = Some(NaN)    score.problems() = []
```

so the layer's gain is `NaN` for the rest of the process and nothing reports it. `MusicLayer::gain_db` is the same
class of caller number this iteration normalised, reaching a **different** field by a different door — the one place
`finite_or` was not applied. That is the next item, not a guess: measured, and left unbuilt because this iteration
shipped a complete change.

**Baseline note.** Nothing was red; the suite had 84 tests.

---

## Iteration 17 — 2026-11-02 — the backend flag does not decide ownership alone

**Change.**

- **`src/play.rs` — a voice is mixer-owned only when a mixer is there to own it.** `Backend` is a public resource whose
  flag a host may set, and `spawn_voice` decided *"the mixer owns playback"* from that flag alone. With `Backend(true)`
  and no `Output`, a voice carried **no `AudioPlayer`** and had **no mixer**: it could not sound, so it never ended, so
  it was never retired — it held its budget slot for the life of the process while `Played` said it had played.
  Ownership is now `backend.0 && output.is_some()`.
- `tests/play.rs` — two tests: the fallback (**fails on the base commit**) and the guard (with an `Output`, the voice
  still carries no `AudioPlayer`, so the two paths cannot both play it). Plus a CHANGELOG line and a media note.

**Why.** Iteration 16's lead was the **two-backend** promise. Probed, a **steal** logs identically on both paths and a
scripted **duck** gives identical `bus_db`/`ducked` (min −8.000 dB on each) — the claim holds. The defect was one level
down: not *do the backends answer the same*, but *can a backend answer nothing at all*. It ranks 1 because a leaked slot
is not a missing sound, it is a budget that never comes back — the failure `cues.yaml` cases 20 and 21 were written to
catch, arriving by a route they do not construct.

**Evidence.** `runs/2026-11-02-17/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/backend{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `Backend(true)`, no `Output` | **`AudioPlayer 0`** — silent, unretirable | `AudioPlayer 1`; `Played` answered |
| `Backend(true)`, with `Output` | `AudioPlayer 0` | **unchanged** — the guard row |
| `Backend(false)`, no `Output` | `AudioPlayer 1` | unchanged |
| a steal, both paths | identical `SirenLog` | unchanged |
| a duck, both paths, 90 frames | identical `bus_db`/`ducked`, min −8.000 dB | unchanged |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost | 0.064 / 0.351 / 0.734 µs | 0.077 / 0.370 / 0.713 µs, load 8.25/9.79/14.97 — within noise, and off the path `perfcheck` costs |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One test defect caught at Fresh eyes: the fallback test
asserted "no `AudioPlayer`" without removing the `Output` the plugin inserts when the `mixer` feature opens a device —
a test whose precondition is "no `Output`" must say so.

**Traps for the command.** (1) A **flag** is not a **fact**: `Backend(true)` says "a mixer started", and a resource a
host can set is not evidence that it did. Read the resource the claim is about. (2) A voice that cannot sound cannot
**end**, so it cannot be **retired**: silent loss and a leaked budget slot are one bug, and the leak is the worse half.
(3) When a test's precondition is "this resource is absent", remove it — with a feature on, the plugin may have
inserted it before the test ran. (4) The guard half of a fix (`AudioPlayer 0` with an `Output`) must be its own test,
or a fix that always inserts a player passes the other one. (5) `hs distill` GPU-blocked (sixteenth iteration).

**Commit.** `26b676b`, pushed to `main`; CI run 41572684531 is **success** on `big-s1` (fmt + clippy + doc, then the
standalone build and test). The Cauldron's `audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test
(84 + 4), serde (91 + 4), mixer (92 + 4), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **Re-time a restarted stem to the bar** — `needs-decision/restart-on-the-bar.md`.
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, sixth pass.** The **two-backend** class is clean for steal and duck, and this iteration closed the "answers
nothing" case. The untested surface left is the **`AudioOutput` trait's own contract**: `set_voice_db` is called by
`update_mix` on **every** voice each frame, but nothing proves a backend that ignores it (as `FakeOutput` does) cannot
desynchronise `Applied`. Next hunt: what `mix.rs`'s `Applied` records versus what the backend was actually told.

**Baseline note.** Nothing was red; the suite had 83 tests.

---

## Iteration 16 — 2026-11-02 — a layer that names a cue which ends

**Change.**

- **`src/music.rs` — `start_layers` does not start a layer whose cue is not a lock `Mode::Loop`.** `MusicScore::problems`
  states the rule (*"names cue `hit`, which is not a loop, so it cannot hold a layer"*); the plugin ignored it. A
  `Mode::Once` cue was started, its voice was retired — a once voice **ends**, and `retire_voices` only skips loops —
  and iteration 15's restart rule asked for it again **every frame**: 10 plays in 10 frames, no `Refused`, no line.
  `start_missing` now reads the cue the layer names **this frame**, skips a cue that is not a loop, keeps the layer in
  `names()`, and writes **one** line in `problems()`' own words. A table edit that makes the cue a loop starts the
  layer on the next frame.
- `tests/music.rs` — three tests, all failing on the base commit; a *"A layer must name a cue that loops"* section; a
  CHANGELOG line; and `docs/loop/needs-decision/not-a-loop-refusal.md` for the `Refusal` variant this does **not** add.

**Why.** The hunt at the end of iteration 15 pointed at the **consumer-side** resources. Probing `SirenChoices` found it
**sound** — a fresh `SirenChoices::new(seed)` reproduces the stream, re-writing the same seed continues it, resetting
the public `rng` alone reproduces it. Probing the rest found three green and one rank-1 red: the asymmetry that made it
rank 1 is that an **unknown** cue was refused with a reason and a **one-shot** cue played at frame rate with no line at
all.

**Evidence.** `runs/2026-11-02-16/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/one-shot-layer{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a `Mode::Once` layer, its sample ending | **10 plays in 10 frames**, `live()` 1, no line | **0 plays**, `live()` 0, one line naming layer and cue |
| a `Mode::Loop` layer (control) | 1 play, `live()` 1 | unchanged |
| a layer naming an unknown cue (control) | refused, `no cue of that name` | unchanged |
| the table fixed to `Mode::Loop` | never starts | starts on the next frame |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost | 0.064 / 0.351 / 0.734 µs at 8/64/256 | unchanged — one table lookup per **voiceless** layer, no per-voice work |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One defect in my own measurement caught at Zoom: the first
probe despawned the layer's voice every frame **for every row**, so the loop control also read 10 plays and proved
nothing; an end is now modelled only for a one-shot.

**Traps for the command.** (1) A restart rule turns *silence* into *rate*: once iteration 15 made a dead voice restart,
any cue that ends became a frame-rate retrigger. A retry must know what "ended" means for the thing it retries.
(2) A control row is only a control if the **stimulus** is withheld from it — the first probe withheld nothing. (3)
`problems()` and the plugin are two paths to one rule; when they disagree, the plugin is the one that plays.
(4) Clippy's 7-argument limit on a Bevy system is Bevy's, not the design's: `play_requests`, `answer` and
`start_pending` already carry the allow. (5) `hs distill` still GPU-blocked (fifteenth iteration).

**Commit.** `ad55c66`, pushed to `main`; CI run 41569616373 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (83 + 4), serde (90 + 4), mixer (90 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md`; the log line is the enforcement
   today, and a host reading `Refused` alone cannot see it.
2. **Re-time a restarted stem to the bar** — `needs-decision/restart-on-the-bar.md` (iteration 15's item 1).
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, fifth pass.** The **resource-contract** class is now exhausted in two directions: what a public resource
*holds* (13–15) and what a consumer resource holds about the crate (this one — `SirenChoices` **sound**). The next
untested surface is the **two-backend** promise: "the two backends answer identically", which `tests/mixer.rs` proves for
play and stop. Nothing proves it for a **steal**, a **duck**, or a **music layer**.

**Baseline note.** Nothing was red; the suite had 80 tests.

---

## Iteration 15 — 2026-11-02 — a music layer's voice dies, and now the layer knows

**Change.**

- **`src/music.rs` — `MusicState` reconciles its voices every frame.** It wrote the entity a layer's `Played`
  reported and never looked again, so a stem stopped by the game, stolen by the budget, or refused at its first play
  left `voice(name)` naming a **despawned** entity, `live()` counting it, and the stem silent for the process's life.
  Three rules now: a dead voice is cleared; a stem that died behind the plugin's back is restarted at the layer's own
  gain; a `StopCue` stays stopped and reads `SILENT_DB`; a refused first start is retried **when a voice frees**.
- **`src/play.rs` — a music layer is unstealable whatever its cue authored.** `Voice::unstealable`'s doc says *"music,
  stingers and dialogue are not stolen"*; the mechanism was `priority >= UNSTEALABLE_PRIORITY`, so a layer authored with
  the default `priority: 0` was called music and treated as a footstep. The budget reads the `MusicVoice` the plugin
  already inserts; the newcomer is refused.
- `tests/music.rs` — four tests, all failing on the base commit; a *"When a stem's voice dies"* section and a
  `CHANGELOG.md` line.

**Why.** Iteration 14's advice was to hunt `SirenSettings`; the hunt found a **contract class** one level up — a
resource that mirrors another system's entities. Iteration 12 was one path of it (a handle that outlived its voice);
this is the mirror itself. The worst row is rank 1: a **stem silently stolen by a UI click**, which R4 and the crate's
own research say must never happen.

**Evidence.** `runs/2026-11-02-15/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/layer-voices{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `StopCue` on a stem | `voice()` `Some(47v0)` — **despawned**; `live()` 1 | `None`, `live()` 0, `layer_db` −80 dB |
| `ui.click` beside a `priority: 0` stem, `max_voices: 1` | `voices ["ui.click"]`, **`track.calm stolen by ui.click`** | `voices ["track.calm"]`, no steals; the click refused |
| a stem whose voice died | `Some(47v0)` → `Some(47v0)`, still dead, `live()` **1** | `Some(47v0)` → `Some(48v0)`, `live()` 1 |
| a refused first start | `live()` **0** forever | `live()` 1; refusals **1 → 1** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.135 µs | 0.064 / 0.351 / 0.734 µs |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two defects caught in my own work: the retry compared
against the count of *pending intents* instead of *voices* (so it never retried — probe 27 caught it), and a
game-stopped layer kept reporting the gain of its last playing frame.

**Traps for the command.** (1) A mirror of another system's entities must be reconciled **every frame**; a `voice:
Option<Entity>` is a claim the world can falsify without telling you. (2) Read *why* a voice is stopping while
`Stopping` is still on it — it despawns with the entity, and a frame later the reason is gone. (3) `voice: Option` is
not truthiness: `live()` counted the dead. (4) A retry condition must count the resource it waits for, not the queue
that asks for it. (5) The Cauldron panel test needs ~14 GB; disk hit 99 % and it failed with `ld: errno=28` — the skill's
own warning. `hs distill` is GPU-blocked (fourteenth iteration).

**Commit.** `fd85856`, pushed to `main`; CI run 41566251355 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (80 + 4), serde (87 + 4), mixer (87 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **Re-time a restarted stem to the bar.** A restart is a new play from sample zero and is out of phase. Needs a
   decision about what a restart means after a gain change: `needs-decision/restart-on-the-bar.md`.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
4. Quantise a **retarget** onto the bar (iteration 14's item 1).
5. A `Refusal` payload that names the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, fourth pass.** The **resource-contract** class has now paid three times: `max_instances`'s sentence (13),
`MusicScore`'s edits (14), `MusicState`'s mirror (15). The remaining unprobed surface of it is the **other direction** —
what a *consumer* resource holds about the crate: `SirenSounds`, `SirenChoices`, and the `Output`/`Backend` a host
installs. `SirenChoices` is a `Resource` a project may reset; check what resetting it mid-stream does to a determinism
promise.

**Baseline note.** Nothing was red; the suite had 76 tests. No case in `cues.yaml` stops a layer's voice or authors a
layer with the default priority.

---

## Iteration 14 — 2026-11-02 — a score edited at runtime reconciles its voices

**Change.**

- **`src/music.rs` — `start_layers` reconciles a changed score's voices, not only its layer list.** A layer **removed**
  from the score had its looping voice left playing forever, ownerless (`retire_voices` skips loops by design); a layer
  **retargeted** to a new `cue` kept the old sample while the new one never started. Both now stop through the crate's
  one fade, marked `Stopping` with the true reason (`"layer removed"` / `"layer retargeted"`), so the log says what the
  score did; a retargeted layer starts its new cue the same frame. A **gain** edit keeps the voice.
- **`MusicState::names()`'s documented score order is restored** after the reconcile — my first version dropped and
  re-appended a retargeted layer and returned `["combat", "calm"]`; the gate caught it.
- `tests/music.rs` — three tests (removal, retarget, gain-keeps-the-voice), two of which fail on the unfixed tree; an
  "Editing the score" section in the music guide; the CHANGELOG and a media note.

**Why.** Iteration 13 named the class that keeps paying: what a **public resource's** contract promises. `MusicScore` is
documented as a file *"a project ships and the Cauldron edits"* — a value that changes at runtime — and the plugin only
ever added to it. Two edits left the score and the sound disagreeing in silence, and one of them (the retarget) plays
the **wrong sample indefinitely**.

**Evidence.** `runs/2026-11-02-14/`: `eval.md` (baseline green; no case edited a score at runtime), `research.md`,
`plan.md`, `perf.md` (the two probes), `review.md`, and `evidence/{before,after}/`.

**Measured.**

| edit | before | after |
|---|---|---|
| remove a layer | `names = ["combat"]`, **2 voices live**, no line | 1 voice, `stopped … layer removed` |
| retarget a layer | the score says `track.b`; **`track.a` plays** and `track.b` never starts | only `track.combat` live; two lines |
| change a gain | eases | same entity, no restart |
| `names()` | `["combat", "calm"]` on the first attempt | `["calm", "combat"]`, score order |
| every rendered cue, the mix, `examples/music` | — | **byte-identical**; *"transitions on the bar: yes"* |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.135 µs | the reconcile runs only on a frame the score changed |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two defects found by the gate in my own work: the first fix
logged every stop as `"stop"` (a `StopVoice` carries no reason), and the second broke `names()`'s documented score order.

**Traps for the command.** (1) `StopVoice` carries an **entity and no reason**, so a system that knows *why* it is
stopping something must mark `Stopping` itself rather than route through `stop_requests` — otherwise the log says
`"stop"` and the reason is lost. (2) A reconcile that drops and re-adds an entry silently changes **order**; check every
accessor that documents one. (3) A `retain` that only filters the state is half a reconcile when each entry owns a
voice: the other half is what the voice does. (4) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB
VLLM engine holds the card), twelfth iteration running.

**Commit.** `0bff015`, pushed to `main`; CI run 41560011538 is **success** on `big-s1`. Gate: build, test (76 + 4), serde (83 + 4), mixer (83 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **Quantise a retarget onto the bar.** A cue change restarts immediately; `When::NextBeat` exists for gain changes.
   Needs a decision about the gap between the old stem's fade and the bar.
2. A method to reconcile a cue's live voices with its cap (iteration 13's item 1; new API, decision first).
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
5. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, third pass.** The handle class is exhausted (iteration 12 closed the only unguarded path; iteration 13 ran
the rest). The **resource-contract** class has now paid twice: `max_instances`' sentence, and `MusicScore`'s. The next
hunt should look at the third resource a project may replace — `SirenSettings` — for a field whose sentence promises
more than the frame does.

**Baseline note.** Nothing was red, and the suite had 73 tests. Cases 15 and 23 drive the score; neither edits it.

---

## Iteration 13 — 2026-11-02 — the cap is a budget on plays, and the docs say so

**Change.**

- **`src/cue.rs` — `Cue::max_instances` now says "may **start**", and why.** It read *"how many of this cue may sound at
  once"*, a standing promise; the crate keeps it at the moment of each play. A table edited under live voices leaves
  them sounding and refuses the next request. `CueTable::insert` — the method a project calls to edit one at runtime —
  points at the same sentence, and `docs/guide/buses.md` says it in a project's words: all three caps are budgets on
  plays, and editing a table under playing sounds is safe.
- `tests/mix.rs` — two tests hold the reading: a lowered cap refuses and does not cut; a raised cap admits. The mutation
  that would adopt the other reading (removing `room_for`'s unstealable guard, the shape a "reconcile by ending"
  implementation has) reddens the first. **No behaviour changed.**

**Why.** Iteration 12's log named the hunt: hold a public handle across a frame and use it as documented. This pass ran
it over every remaining carrier — `Play::on` in both despawn orders, `MusicState::voice`, `SirenChoices`, the `Samples`
handle — and all were safe; the audit of every `commands.entity(..)` confirms iteration 12 closed the only unguarded
path. What the hunt did find was a **documented promise with two readings**, and the written one was the one the crate
does not implement.

**Evidence.** `runs/2026-11-02-13/`: `eval.md` (baseline green; case 7 asserts the cap only at play time), `research.md`,
`plan.md`, `perf.md` (the probe and the mutation), `review.md`, and `evidence/{before,after}/` with both render trees, the
spectrograms and the loudness numbers.

**Measured.**

| sequence | observed | documented now |
|---|---|---|
| two voices at `max_instances: 2`, table edited to 1, third play | refused `VoiceLimit`; **both voices still sounding**; nothing stopped or logged | the cap governs the next play; live voices live out their length |
| cap raised from 1 to 2 under one live voice | the next play is `Played` | unchanged |
| the rejected design | removing `room_for`'s unstealable guard reddens the new test | measured |
| every rendered cue, the mix, and `examples/music` | — | **byte-identical**; *"transitions on the bar: yes"* |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found two defects in my own tests — priority-0
voices meant the lowered cap *stole* rather than refused, and a second test read a `Played` left buffered by the previous
play — both the loop's old traps, both fixed.

**The alternative, rejected with a reason.** Reconciling live voices would despawn a voice the game did not stop, on a
frame it did not choose, and log a stop it did not request: three surprises to replace one, plus rules for which voice
goes and for a table edited twice in a frame. A feature with its own decision, not a doc fix.

**Traps for the command.** (1) A cap that can **steal** never refuses: to test a refusal, the voices must be unstealable
or the test asserts the wrong path. (2) Drain `Played`/`Refused` between plays, or the next assertion counts the
previous request's message. (3) Two readings of one sentence can both be defensible; the repair is to write down which
one the code takes and hold it with a test, not to change the code to match the looser reading. (4) `hs distill` is still
GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), eleventh iteration running.

**Commit.** `c53f030`, pushed to `main`; CI run 41559273528 is **success** on `big-s1`. Gate: build, test (73 + 4), serde (80 + 4), mixer (80 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **A method to reconcile a cue's live voices with its cap**, for a project that wants the count enforced — as
   something a game calls, never as a frame the crate chooses. New public API, so a decision file first.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
4. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
5. `Completes` is public and can never be true — `needs-decision/remove-completes.md` (its `Entry` arm can carry item 1
   of iteration 10's note).

**The hunt, after two passes of it.** Iteration 12 found a crash by holding a handle across a frame; this pass ran the
same shape over every other carrier and found nothing, then found a contract gap by asking what a **public field's
sentence** promises. Both hunts are now written into the command: the handle class is exhausted, and the contract class
is the one that keeps paying.

**Baseline note.** Nothing was red, and the suite had 71 tests. Case 7 asserts the polyphony cap only at play time; no
case edits a table under a live voice.

---

## Iteration 12 — 2026-11-02 — a stop for a sound that already ended

**Change.**

- **`src/mix.rs` — `stop_requests` resolves the entity against the live voice query before commanding it.** It took the
  id straight from the `StopVoice` message and called `commands.entity(entity).insert(..)`; `retire_voices` despawns a
  one-shot the frame it ends, and `Played::entity` is documented as the handle a game keeps **for `StopVoice`** — so
  holding it one frame too long hit Bevy 0.20's default error handler, which **panics**:
  `Entity despawned: … its index now has generation 1`. Now a stop for a voice that has gone is a no-op, and a second
  stop for a voice already fading is skipped so a fade is begun once.
- `tests/play.rs` — three tests: the stale handle (**panics inside `app.update()`** on the unfixed tree), a second stop,
  and `StopCue` so the fix cannot narrow the name path. `cues.yaml` case 30; a "Stopping a sound" section in the cues
  guide; the CHANGELOG and a media note.

**Why.** Iteration 11's log said the most productive next pass was **a new cue case that turns something red** — and one
did, in the first probe. This is a crash reachable from the crate's own documented workflow and its own retirement path:
`Played` hands out an id, the crate invalidates it on its own schedule, and using it as documented took the game down.
Every other `commands.entity(..)` in the crate takes an entity from a live query and cannot be dead; this was the one
path where the id came from a **message**.

**Evidence.** `runs/2026-11-02-12/`: `eval.md` (baseline green; the gap — case 23 stops a live loop, no case stops an
ended voice), `research.md`, `plan.md`, `perf.md` (the panic and its absence), `review.md`, and `evidence/{before,after}/`.

**Measured.**

| sequence | before | after |
|---|---|---|
| play a one-shot, let it end, `StopVoice(handle)` | **panic: `Entity despawned … generation 1`** | no-op; nothing logged |
| play a loop, `StopVoice` | fades | unchanged |
| `StopVoice` twice for a fading voice | fade re-armed | skipped; one fade, one line |
| `StopCue` with two voices | both stopped | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.069 / 0.628 / 1.391 µs | one `Query::get` per stop request, an event not a frame cost; load 8.0 vs 2.4 is the difference from iteration 11 |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The gate found two defects in my own work — a leftover import
and a scaffold expression in a test — both fixed; and the commit itself shipped a throwaway probe by mistake, removed in
the commit that follows it (`975f12b`).

**Traps for the command.** (1) **Check `git status` before committing after a hunt**: `examples/probe16.rs` was deleted
from the working tree but staged with `git add -A`, and an example directory is part of the published crate. (2) A
command on an id that came from **outside** the world needs a guard; a command on an id from a **query** does not. Grep
`commands.entity(` and check where each entity came from — that is the whole audit. (3) The regression test's value is
that it **panics**, not that it asserts a value: the unfixed tree fails inside `app.update()`, which no assertion can
express. (4) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), tenth
iteration running.

**Commit.** `d551abb` + `975f12b`, pushed to `main`; CI run 41558456518 is **success** on `big-s1`. Gate: build, test (71 + 4), serde (78 + 4), mixer
(78 + 4), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.** The five decision files are unchanged; one minor can carry four of them.

1. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
2. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
3. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
4. `Completes` is public and can never be true — `needs-decision/remove-completes.md`.
5. A log line for a position a non-spatial cue ignores — `needs-decision/non-spatial-position.md` (its `Entry` arm can
   land with item 4).

**The hunt that worked, written down.** Iteration 11 found nothing in six probes because it tested what the crate
*decides*; this one found a crash in the first probe because it tested **what a game does with what the crate hands it** —
keep the entity, use it later. That is the class of case the table was missing, and the next hunt should start there:
take a public handle, message or resource, hold it across the frame the crate changes it, and use it as documented.

**Baseline note.** Nothing was red, and the suite had 68 tests. Case 23 stops a **live** loop; nothing stopped a voice
that had **ended**, which is the only sequence that reaches the panic.

---

## Iteration 11 — 2026-11-02 — the rule written the way it reads

**Change.**

- **`tests/mix.rs` — two tests state `UNSTEALABLE_PRIORITY` directly.** It was proved through
  `the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue. Now a **priority-0**
  request against a **priority-200** voice is refused with nothing spawned, the voice still live and no `stolen by`
  line; and a stealable voice still plays beside an unstealable one, so neither can pass vacuously.
- `cues.yaml` case 9 names the stealable-request direction as its observation and cites Gregory §14.5.8.4;
  `CHANGELOG`, media note. **No `src/` file changed at all.**

**Why.** Two things, and the second is the honest one.

First, `DuckClock.started` — iteration 10's next-5 head — **is not a bug**: measured, 50 fires over two rules leave two
entries, because the map is keyed by rule, not by fire, and `duck_db` reads only pairs still in `settings.ducks`.

Second, with that closed and five decision-blocked items left, this pass tested **six** candidate rank-1 defects and
found none: the map above; a stop on the play's own frame (still fades — `update_mix` writes `Applied` before
`stop_requests`); `Played.gain_db` against the audible chain (−11.00 dB both); a stealable request taking an unstealable
voice; a bus cap above the table's (clamped); `SirenLog` at capacity. What remained was legibility, and that is what
shipped.

**Evidence.** `runs/2026-11-02-11/`: `eval.md` (baseline green; the six candidates and their measurements),
`research.md` (**corrected mid-run** — my first premise was wrong), `plan.md` (corrected the same way), `perf.md` (the
mutation), `review.md`, and `evidence/{before,after}/` with the render trees, spectrograms and loudness.

**Measured.**

| candidate | result |
|---|---|
| `DuckClock.started` grows with table swaps | **two** entries after 50 fires over two rules |
| a stop on the play's own frame cuts | fades for 0.48 s; `Applied` is written before `stop_requests` |
| `Played.gain_db` vs the audible chain | −2 −3 −6 = **−11.00 dB**; `Applied` −11.0 |
| a stealable request takes an unstealable voice | refused; nothing stopped |
| a bus cap above the table's | clamped with `.min(total)` |
| `SirenLog` at capacity | keeps 256, drops the oldest |
| **the mutation: `steal` ignores `unstealable`** | **three** tests red — both new ones **and** the old table-budget test |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT — for what this is: a legibility change plus six negative
results, not a repair.

**A premise corrected by measurement.** I wrote, and briefly believed, that the unstealable promise had an untested
half. The mutation disproved it in one run: the existing test reddens too, because its second request then steals its
own older voice. `research.md` and `plan.md` carry the correction rather than the original claim, because the loop's
own rule is that a claim without a measurement is a guess.

**Traps for the command.** (1) A mutation is the cheapest way to find out whether a test covers a rule — and it can
disprove the reason you wrote the test. Run it *before* claiming the gap. (2) A test-only iteration still owes the
evidence rules: the `before` tree here is the after-tree's binary, because `src/` is unchanged, and `perf.md` says so
instead of presenting a lower number (load 2.4 against 4.7) as an improvement. (3) `hs distill` is still GPU-blocked
(`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), ninth iteration running.

**Commit.** `ef2f91b`, pushed to `main`; CI run 41557694429 is **success** on `big-s1`. Gate: build, test (68 + 4), serde (75 + 4), mixer (75 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.** All five are public-enum decisions for the owner; one minor can carry them together.

1. **A `Refusal` variant for a stinger on the wrong bus** — `needs-decision/stinger-refusal.md`.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **A log line for a position a non-spatial cue ignores** — `needs-decision/non-spatial-position.md`, whose `Entry` arm
   could land with item 4.

**A note on the loop's run.** Eleven iterations have taken the crate from "a 32-voice budget that silently locks out on
the mixer path" to "every rank-1 defect the suite and the probes can find is closed". The remaining work is not
engineering the loop can do alone: four of the five are one `match` arm each, and the fifth is one field. The next
iteration should start with the owner's answer to any of them, or with a new cue case that turns something red.

**Baseline note.** Nothing was red, and the suite had 66 tests. Six candidates were probed and none was a defect, which
is the finding this entry records.

---

## Iteration 10 — 2026-11-02 — the mix's buses are this frame's buses

**Change.**

- **`src/mix.rs` — `update_mix` iterates the buses `bus_names()` returns, not `settings.buses`.** With `buses: []` the
  crate accepted a cue on `Effects` (that is what `answer`, the panel and `bus_names()` all promise) and then mixed it at
  **−80 dB** — silence with no refusal, on the crate's own default-shaped table.
- **`src/mix.rs` — `SirenMix::db` and `ducked` are pruned to this frame's buses** before the bus loop. A project that
  swapped `SirenSettings` kept every bus from the previous table at its last level: `bus_db` answered for a bus that did
  not exist, `deepest_duck()` named one forever, and a removed bus's duck froze at its deepest. The prune runs before the
  loop, so a bus that **stayed** still reads its previous value and eases instead of steppng.
- `tests/mix.rs` — three tests: the swap (fails on the unfixed tree), the empty list (fails on the unfixed tree), and the
  non-regression a wholesale rebuild would break. `examples/render` prints what `bus_names()` answers for a table and for
  a table of none; `cues.yaml` case 29; the buses guide, the CHANGELOG and a media note.

**Why.** The five next-5 items are all public-enum decisions the owner has not answered, so the pass hunted for a
rank-1 defect instead of idling — and the second half turned up while testing the first, which is the loop's own point
about a test that measures the thing rather than a plausible neighbouring thing. `buses: []` at −80 dB is wrong output,
silent, with no refusal: the exact class this loop ranks first.

**Evidence.** `runs/2026-11-02-10/`: `eval.md` (baseline green; the table's gap — no case swaps the table or uses `[]`),
`research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both render trees, the bus-list line,
the spectrograms and the loudness numbers.

**Measured.**

| measurement | before | after |
|---|---|---|
| `mix.db` after a 4→2 bus swap | all four, ghosts included | `[Effects, Music]` |
| `bus_db("Ambience")`, a bus not in the table | **−3.3 dB** | **−80.0 dB** (`SILENT_DB`) |
| `deepest_duck()` | `Some(("Ambience", −12.0))` forever | `None` |
| `buses: []`, a cue on `Effects` | accepted, **mixed at −80 dB** | accepted, mixed at 0 dB on the crate's four |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.065 / 0.354 / 1.389 µs | two map prunes over ≤ the bus count; not on the per-voice path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The gate found two defects in my own work — a `dt = 0` in a
test of mine (the crate's documented trap, in a test written two iterations after writing it down) and a dead-code
warning on a `mixer`-only helper — both fixed before the verdict.

**Traps for the command.** (1) Disk reached **674 MB** and `ld` failed with `errno=28` — the loop's documented trap,
third time in this project. 15 GB came back from build directories this loop created (`/tmp/seedprobe` 6.6 GB,
`/tmp/siren-pub` 7.4 GB), never from the project's `target/`. (2) A `BTreeMap` written by insertion is a cache with no
owner: `voices` was already rebuilt wholesale, and these two maps beside it were the exception, which is why only they
went stale. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), eighth
iteration running.

**Commit.** `29a431c`, pushed to `main`; CI run 41557018186 is **success** on `big-s1`. Gate: build, test (66 + 4), serde (73 + 4), mixer (73 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `bus_rows` iterates the table's buses, so it was never shown the ghosts — the crate's own accessors
were.

**Next 5.**

1. **`DuckClock.started` has the same staleness** as the mix maps had: keys are `(source, target)` pairs and a pair whose
   buses left the table is inert but never removed. Cheap to prune beside the maps.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
4. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
5. `Completes` is public and can never be true — `needs-decision/remove-completes.md`.

**Baseline note.** Nothing was red, and the suite had 63 tests. Every case used the **default** table; none swapped it
or listed no buses, so the two places that disagree about which buses exist were both invisible.

---

## Iteration 9 — 2026-11-02 — a stinger's duck waits for a sound that plays

**Change.**

- **`src/music.rs`, `src/lib.rs` — the duck is armed on `Played`, not on the request.** `play_stingers` wrote the `Play`
  and armed the duck in the same breath while `Play` is answered later in the frame, so a refused stinger left the
  `Music` bus down for a sound nobody heard — and the log said both. A new `arm_stinger_ducks` runs after `Play` and
  before `Mix`, arming the duck for a stinger that was answered and dropping one that was refused; the dip still lands
  on the frame the stinger fires.
- **A cue not in the table is refused as `UnknownCue`**, instead of inheriting `Music` through `unwrap_or("Music")` and
  passing the bus check. **Stinger refusals write a `Refused` message**, not only a log line.
- `tests/music.rs` — four tests: unknown cue, missing sound (both fail on the unfixed tree), a real stinger ducking on
  the frame it fires, and a wrong-bus stinger. `examples/render` gains the unknown-cue refusal; `cues.yaml` case 28;
  `needs-decision/stinger-refusal.md`; the music guide, the CHANGELOG and a media note.

**Why.** Rank 1 at its sharpest: a duck that corresponds to no sound. The log was the proof — *"stinger ducked Music by
-9.0 dB"* on the line above *"refused … no handle …"* — two records of one request disagreeing, which is exactly what
the crate's refusals exist to prevent. Gregory §14.6.3 and the crate's own R3 both say a duck is driven by a voice that
starts; the dialogue path already obeyed that, and stingers were the one exception.

**Evidence.** `runs/2026-11-02-09/`: `eval.md` (baseline green; case 16 proves a stinger ducks and no case asks what
happens when it cannot), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the new refusal file, the spectrograms and the loudness numbers.

**Measured.**

| case | before | after |
|---|---|---|
| cue not in the table | **Music −9.00 dB**, `UnknownCue`, log says it ducked | Music 0.00 dB, `UnknownCue`, **one** line |
| a sound with no handle | **Music −9.00 dB**, log says ducked **and** refused | Music 0.00 dB, refusal only |
| a stinger that plays | −9.0 dB on its frame | unchanged |
| a stinger on another bus | refused, no duck | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.063 / 0.367 / 1.347 µs | not on this path; one empty `Vec` and two drained buffers per frame |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass caught a regression I nearly shipped: the
wrong-bus branch briefly reused `Refusal::UnknownBus`, whose sentence (*"bus `Effects` is not in the mix table"*) is
**false** — `Effects` is in the table. Reverted to the true prose line, and the variant that would say it properly is
`needs-decision/stinger-refusal.md`.

**Traps for the command.** (1) A bug with two independent halves needs **both** reverted to prove the tests: reverting
only the ordering left the unknown-cue test passing, because the table lookup half still refused it. (2) `render.txt`
and the rendered audio cannot show an ordering fix — the panel settles to the same value once the frame is over; the
tests are the evidence, and the log is the *symptom*. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB;
a 14.8 GB VLLM engine holds the card), seventh iteration running.

**Commit.** `64fce41`, pushed to `main`; CI run 41556188200 is **success** on `big-s1`. Gate: build, test (63 + 4), serde (70 + 4), mixer (70 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; the panel prints `SirenLog` lines, so a stinger that cannot play now shows one line where it showed
two — an improvement it gets without a consumer change.

**Next 5.** Four of the five are now public-enum decisions that one minor can carry together:

1. **A `Refusal` variant for a stinger on the wrong bus** — `needs-decision/stinger-refusal.md`.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **A log line for a position a non-spatial cue ignores** — `needs-decision/non-spatial-position.md`, whose `Entry`
   arm could land with item 4.

**Baseline note.** Nothing was red, and the suite had 59 tests. Case 16 proves the duck happens; nothing asked what
happens when the stinger cannot play, which is why the contradiction sat in the log unread for six iterations.

---

## Iteration 8 — 2026-11-02 — a non-spatial cue is never handed to the panner

**Change.**

- **`src/play.rs` — one predicate decides placement.** `spawn_voice` set Bevy's `PlaybackSettings.spatial`, and
  attached a `Transform`, from `request.position.is_some()` alone, while `answer` applied attenuation under
  `cue.spatial && position.is_some()`. A cue authored `spatial: false` and played with `.at(..)` therefore went through
  `rodio`'s panner, which applies `min(1, 1/d²)` as well as a pan: **−38.7 dB at 8 m**, while `Played.gain_db` read
  **0.00 dB**. `Started.spatial` now carries the resolved fact and all three places read it.
- `tests/play.rs` — four tests: the mismatch (fails on the unfixed tree), and the three cases it must not overshoot
  (spatial at a position, spatial without one, spatial without a listener). Two `PlaybackSettings` assertions are gated
  to Bevy's path, because the `mixer` build has no settings component — the two backends must answer identically, and
  the test had only been testing one. `tests/model.rs` pins the panner's own law at 0.0117 linear / −38.7 dB.
- `examples/render.rs` — a `panner:` line giving that law in dB; `cues.yaml` case 27; the spatial guide, the CHANGELOG
  (with who could notice) and a media note; `needs-decision/non-spatial-position.md`.

**Why.** Rank 1 — wrong output — and its worst sub-class: a `Played` whose own `gain_db` disagrees with the sample, so a
game's meter believes a sound is at 0 dB while it plays 38 dB down. The crate's guide already states the principle the
code broke: *"playing it flat, or at the origin, is a different sound from the one the game asked for."* The three
higher-listed next-5 items are all decision-blocked, so this was the only rank-1 item buildable today.

**Evidence.** `runs/2026-11-02-08/`: `eval.md` (baseline green; the fourth combination of the spatial table not covered),
`research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both render trees, the panner line,
the spectrograms and the loudness numbers.

**Measured.**

| request | before | after |
|---|---|---|
| non-spatial cue, `.at(8 m)` | panned, **−38.7 dB audible**, `Played.gain_db` 0.00 | flat at its own gain, no `Transform`, no panner |
| spatial cue, `.at(8 m)` | −6.02 dB, compensated, placed | unchanged |
| spatial cue, no position | flat at its own gain | unchanged (documented) |
| spatial cue, no listener | `SpatiallyMute` | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.063 / 0.354 / 1.343 µs | not on this path; one `bool` computed once per play |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass caught a real defect in my own tests: both
`PlaybackSettings` assertions panicked under `--features mixer`, where the crate's own graph owns playback and there is
no settings component. Gated, and the rule that the two backends answer identically is now honoured by the test rather
than assumed.

**A number worth keeping.** The panner's own law at 8 m on-axis is **0.0117 linear, −38.7 dB** — computed from
`model::backend_spatial_gain`, printed by `examples/render`, and pinned by a test. That is the size of the silence a
"flat" cue was being handed while the log said 0 dB.

**Traps for the command.** (1) `render.txt`'s `panner:` line reads the same in both trees **by design** — it computes the
law, not the routing decision. It is evidence of magnitude; the test is evidence of the fix; confusing them is exactly
the trap iteration 7 hit. (2) A test that reads `PlaybackSettings` only compiles and passes on Bevy's path; under
`--features mixer` it panics on a missing component, which is not a crate bug. (3) `hs distill` is still GPU-blocked
(`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), sixth iteration running.

**Commit.** `f75c633`, pushed to `main`; CI run 41555323150 is **success** on `big-s1`, and so is the docs commit that follows it. Gate: build, test (59 + 4), serde (66 + 4), mixer (66 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `PlaybackSettings` is not something the panel reads, and `Played.gain_db` is now true rather than
false for this case, which can only improve what it prints.

**Next 5.**

1. **A log line for an ignored position** — `needs-decision/non-spatial-position.md`, option 2, which shares its `Entry`
   arm with `remove-completes.md`. One edit can land both.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **The mixer's decode error is discarded** (`Decoder::new(..).ok()?`); carrying it into the refusal would say *why* a
   file failed. Same decision as item 2.

**Baseline note.** Nothing was red, and the suite had 54 tests. Case 14 proves a spatial cue at two distances and case
13 the no-listener refusal; no case reached the fourth combination — `spatial: false` **with** a position — which is the
one that lied.

---

## Iteration 7 — 2026-11-02 — a voice slot is reused, not appended

**Change.**

- **`src/mixer/mod.rs`, `src/model.rs` — the mixer's per-bus slot list is bounded.** `play` pushed a `VoiceTrack` (a
  `rodio::Player` and its queues) per play and never removed one, so a game on the crate's own mixer retained a player
  for every sound it had ever played. It now reuses the first slot whose voice has ended. An index packs a **slot and
  a generation** (`model::pack_voice`, `unpack_voice`, `VOICE_SLOT_BITS`), bumped on each hand-out, and every
  index-taking method goes through one `occupant_mut` so a stale `MixerVoice` is a no-op rather than a cross-wire.
  `MixerBackend::voices()` is documented as *slots*, not sounds played. **No public type or trait signature changed.**
- `tests/mixer.rs` — a test that drives the **real** backend through 200 one-shots and asserts slots bounded, samples
  started and 200 distinct names *together*; it returns early with a sentence where no device exists. `tests/model.rs`
  pins the encoding at both range ends. `tests/helpers.rs` gains a real 64-sample WAV.
- `cues.yaml` case 26; the buses guide, the CHANGELOG and a media note.

**Why.** Iteration 2 found this and deferred it ("a memory leak of one `Player` handle per one-shot … what a second
iteration's 'compact the voice list' item is for"). Five iterations later it was still there, and it is the one cost in
the crate that grows with how long a game is played: the budget bounds **concurrent** voices, and this list counted
**cumulative** ones. Gregory §14.5.8.4 calls the voice count fixed; here it was not. The other rank-1-class items are
all decision-blocked, so this was the only one buildable today.

**Evidence.** `runs/2026-11-02-07/`: `eval.md` (baseline green; the gap between case 21's budget release and the
backend's own list named), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the spectrograms and the loudness numbers.

**Measured** (real audio device, `MixerBackend::open`):

| | before | after |
|---|---|---|
| 400 one-shots, all ended: slots held | **400** | **6** |
| 200 one-shots: slots held | 200 | ≤ 32 asserted (2 measured) |
| 200 one-shots: distinct indices handed out | 200 | 200 — each hand-out is a fresh *name* |
| a stale index's `finished` / gain / stop | moved the slot's new occupant | no-op; `finished` says ended |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.354 / 1.263 µs | not on this path; `perfcheck` drives neither the mixer nor the music |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect in my own work: the first
probe I wrote for `examples/budget` used a fake output handing out its own indices, so it measured the crate's entity
side — never the leak — and would have printed the same line on both trees. Deleted; the measurement that matters came
from the real backend, which this machine can open.

**Traps for the command.** (1) A fake `AudioOutput` cannot measure `MixerBackend`: the leak is the real graph's slot
vector. Test the real backend where a device exists and skip with a sentence where it does not. (2) Bounding slots by
**reusing indices** would be a worse bug than the leak — a stale `MixerVoice` would move another voice's gain — so the
test asserts *slots bounded* and *names distinct* in the same breath; either alone passes vacuously. (3) `hs distill`
is still GPU-blocked (`bge-m3` wants 5000 MB behind a 14.8 GB VLLM engine), fifth iteration running; the rule is
argued from the vendored `rodio` source and Gregory §14.5.8.4, both already cited here.

**Commit.** `32f0ce3`, pushed to `main`; CI run 41554421622 is **success** on `big-s1`. Gate: build, test (54 + 4), serde (61 + 4), mixer (61 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `MixerBackend::voices()` is unused there, so the one changed accessor reaches no consumer.

**Next 5.**

1. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`; a new
   `match` arm for a consumer, so the owner's call.
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **The mixer's decode error is discarded** at `mixer/mod.rs` (`Decoder::new(..).ok()?`); carrying it into the refusal
   would say *why* a file failed. Same decision as item 1.
5. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question.

**Baseline note.** Nothing was red, and the suite had 53 tests. Cases 20–21 prove the **budget** frees a voice; none
could see the **backend's** own list, which is why five iterations of green runs sat on top of a 400-slot leak.

---

## Iteration 6 — 2026-11-02 — a refused sample names the sound, not the cue

**Change.**

- **`src/playback.rs` — a decode failure names the sound key.** `start_pending` reported
  `MissingSound { sound: voice.cue }` — the cue's name in a field documented as the key a project maps to a file — so a
  corrupt `step_03.wav` sent a person looking for `footstep.grass`. It now recovers the key from the same cue and
  variant `answer` chose (`key_of`), and reads `Res<CueTable>` to do it. No public type, field or message changed.
- **`src/play.rs` — one sentence for the variant's two causes:** *"no handle or undecodable bytes for sound
  `step_03`"*, with the doc saying which two call sites produce it.
- `examples/budget.rs` — a `missing-sound.txt` probe that runs both causes side by side in the real ECS;
  `tests/mixer.rs` — a `refuses_samples` mode on the fake output (the same `None` the real mixer returns when
  `rodio::Decoder::new` errors) and two tests; `tests/play.rs` — the sentence; `cues.yaml` case 25;
  `needs-decision/undecodable-sound.md`; the cues guide, the CHANGELOG and a media note.

**Why.** Top of iteration 5's next-5, and its own class: a refusal confidently naming the wrong subject, in the crate
whose refusals exist to name what is missing. Found by iteration 5's probe, which fed four zero bytes to the mixer and
got `no handle for sound \`line\`` for a cue whose key was also `line` — a coincidence that hid it from every test.

**Evidence.** `runs/2026-11-02-06/`: `eval.md` (baseline green; the table's gap named — case 11 covers only the
missing-handle cause), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the two-cause probe and the cue spectrograms.

**Measured.**

| cause | before | after |
|---|---|---|
| a key with no handle | `sound: "step_03"` (true) | `sound: "step_03"` |
| a key whose bytes do not decode | **`sound: "footstep.grass"`** (the cue) | `sound: "step_03"` |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.059 / 0.329 / 0.827 µs | not on this path; the machine's load was 6.2 against 2.7 in iteration 5, which is the whole difference |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect in my own work: a third test
tried to resolve a default handle's id so a waiting voice would reach the code under test, and `Assets::insert` on that
id does not make `assets.get` succeed — the test proved nothing on any tree and was removed rather than contorted.

**Traps for the command.** (1) A default `Handle::<AudioSource>` **never** resolves, however you insert into `Assets`
afterwards: a test that wants "bytes arrive later" must build a real handle and hold *that*. (2) A cue and its key
having the same name (`line`/`line`, as in `examples/budget`) hides this whole class of bug — the probe's first run
looked correct for exactly that reason. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM
engine holds the card), so the rule is argued from the vendored `bevy_asset`/`rodio` sources and the crate's own R6.

**Commit.** `409eac6`, pushed to `main`; CI run 41553622842 is **success** on `big-s1`. Gate: build, test (53 + 4), serde (60 + 4), mixer (59 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; this change's host effect is a corrected `Entry::Refused` sentence, which the panel prints verbatim.

**Next 5.**

1. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`. A new
   `match` arm for a consumer, so not decided here; option 4 in that file (log the cause) is one line and needs no
   answer.
2. **A payload that also names the cap** — `needs-decision/refusal-names-its-cap.md`.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **The mixer's decode error is discarded** at `mixer/mod.rs:147` (`Decoder::new(..).ok()?`); carrying it into the
   refusal would tell a project *why* the file failed, not only which one. Wants the same decision as item 1.
5. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.

**Baseline note.** Nothing was red, and the suite had 52 tests. Case 11 proves the missing-handle cause; no case could
reach the decode cause through `SirenSounds` alone, which is why iteration 5's probe found it by accident.

---

## Iteration 5 — 2026-11-02 — a refusal names the bus only when the bus refused

**Change.**

- **`src/play.rs` — the refusal names the cap that refused.** `room_for` has three hard caps and returned one
  uninformative `None` from all of them, so `answer` always blamed the cue's bus. It now returns
  `Result<Vec<Entity>, Cap>` with a private `Cap::{Cue, Bus, Table}`; `bus: Some(name)` only for the bus's own cap,
  `bus: None` otherwise — which is what the field's own doc says (*"the bus that had no room, when one did"*).
  No public type, field or message changed.
- `examples/budget.rs` — a **`refusals.txt` probe** that forces each of the three caps in the real ECS and writes the
  payload and the sentence; a real WAV per cue, because dummy bytes are refused by the decoder (`MissingSound`) before
  the budget is ever consulted.
- `tests/mix.rs` — four tests: table and cue caps name no bus (both fail on the unfixed tree), a full bus still names
  it (passes on both, so the fix cannot overshoot), and each payload shape has exactly one sentence.
- `examples/render.rs` — the three `VoiceLimit` sentences; `needs-decision/refusal-names-its-cap.md`; `cues.yaml` case
  24; the buses guide, the CHANGELOG (with the host consequence) and a media note.

**Why.** The loop's first criterion, and worse than a silent failure: a **confidently wrong** one. The probe showed a
table-budget refusal accusing `Ambience` for a cue on `Effects` — a bus with a free slot, not even the cue's own.
Cases 7–9 each asserted `matches!(reason, VoiceLimit { .. })`; the wildcard was the hole, and no case read the payload.

**Evidence.** `runs/2026-11-02-05/`: `eval.md` (baseline green; the table's gap named), `research.md`, `plan.md`,
`perf.md`, `review.md`, and `evidence/{before,after}/` with the two render trees, the refusal texts and the budget
probe.

**Measured.**

| cap that refused | before | after |
|---|---|---|
| the table's `max_voices: 1`, two buses with room | `Some("Ambience")` → ``no voice free on bus `Ambience` `` | `None` → `no voice free` |
| the cue's `max_instances: 1`, its bus at 0 of 8 | `Some("Effects")` | `None` → `no voice free` |
| the bus's `max_voices: 1` | `Some("Effects")` | `Some("Effects")` — unchanged, and now the only one |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | — | 0.022 / 0.096 / 0.667 µs — `room_for` runs per request and only its refusal changed; the harness does not drive it |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect **in my own evidence
harness** (four zero bytes as a sample, so the decoder refused before the budget did — every line read
`MissingSound { sound: "line" }`, and the harness was measuring the wrong subsystem), and it exposed a second real
refusal bug, logged rather than folded in.

**A number worth keeping.** The before tree named a bus in **every** refusal, and the first row names a bus the cue
does not play on. "No voice free" is less informative but true; the diagnostic version needs the cap in the payload,
which is the decision file.

**Traps for the command.** (1) `examples/render` **hardcodes** its refusal strings, so it cannot show a change to a
refusal — the evidence for this iteration is `examples/budget`'s ECS probe, and `render`'s identical refusal files are
expected, not a failure. (2) An app that inserts a dummy `AudioSource` measures `start_pending`'s decoder, not the
budget: use a real WAV. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the
card), and the metadata API returned no abstract for the instance-limiting papers, so the rule is argued from the
crate's own R4 and Gregory §14.5.8.4's *per-group limits*, which this repository already cites.

**Commit.** `3d158ae`, pushed to `main`; CI run 41552664607 is **success** on `big-s1`. Gate: build, test (52 + 4), serde (59 + 4), mixer (56 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; it was not run, and this change's host-visible effect is a different `Entry::Refused` sentence —
the panel prints it verbatim, so it improves without a consumer change.

**Next 5.**

1. **`start_pending` reports a decode failure as `MissingSound { sound: <cue> }`** — a cue's name in a field documented
   as the sound *key*. A project greps its asset list for `footstep.grass` and finds nothing. Found by this
   iteration's probe.
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`, with the three sentences option 2
   would print.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.
5. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.

**Baseline note.** Nothing was red, and the suite had 48 tests. The defect was invisible because every test that
reaches a `VoiceLimit` matched it with `..`: the kind was asserted, the content never.

---

## Iteration 4 — 2026-11-02 — a crossfade takes the time it was given

**Change.**

- **`src/music.rs`, `src/model.rs` — `MusicLayers::fade_s` is honoured.** It was stored in `Pending` and read by
  nothing; `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms.
  `model::tau_for_fade` derives the constant from `approach`'s own definition — `fade_s / ln(1000)`, an arrival within
  one part in a thousand — and each `LiveLayer` carries the fade its own change gave it.
- **`move_layers` holds a layer's gain on a frame with no elapsed time.** `approach` returns its target for `dt <= 0`,
  which is right for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend
  the whole fade in one step. The tests found this; it is fixed in the crate, not in the tests.
- `examples/render.rs` — a **layer-fade plot** (the same move under `MIX_TAU_S` and under an authored 0.8 s) and its
  caption in `render.txt`; `tests/model.rs` and `tests/music.rs` — four tests, one of which fails on the unfixed tree;
  `cues.yaml` case 23; the music guide, the CHANGELOG and a media note.

**Why.** The top of iteration 3's next-5, and the same class as its two predecessors: a number a person wrote whose
sentence the code did not honour, silently. Rank 2 in the loop's order (music transitions and fades) but rank 1 in
practice — the field is public, documented and already in every project's builder chain.

**Evidence.** `runs/2026-11-02-04/`: `eval.md` (baseline green; the one red case is the fade), `research.md`,
`plan.md`, `perf.md`, `review.md`, and `evidence/` with both render trees, the fade plot and its zoom, and the cue
spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| 0.3 s into an authored 0.8 s fade | **−9.13 dB** (settled) | **−13.6 dB** (arriving) |
| the same fade at 0.85 s | −9.0 dB | −9.07 dB → its target |
| `fade_s(0)` at 0.5 s | −9.0 dB | −9.0 dB (default easing unchanged) |
| a layer the change does not name | — | keeps its own fade |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.019 / 0.103 / 0.684 µs | the same — the harness does not drive the music systems |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot**: a
fixed `0 dB` vertical scale that pinned a fade from silence to the floor, a marker labelled "duck fires" on a plot
with no duck, and a t=0 marker drawn on the y axis where its label collided. All fixed before the verdict.

**A number worth keeping.** The derivation is not a fitted constant: `approach` reaches `1 − e^(−t/τ)`, so "arrive
within a thousandth at `fade_s`" is `τ = fade_s / ln(1000)`, and `tests/model.rs` asserts both halves of that —
it has arrived at `fade_s`, and it has **not** arrived at `fade_s / 2`.

**Traps for the command.** (1) A headless test's **first** update has `dt = 0` unless `advance()` ran first, so a test
that writes a message and immediately updates tests the step branch of `approach`, not the fade. Every existing music
test advanced first; the new ones must too. (2) `perfcheck` cannot see a change on the music path — say so in
`perf.md` rather than presenting identical numbers as a measurement. (3) An evidence harness that draws *both* the old
and the new curve prints the same line whichever tree it was built from; that identity is by design and is not
evidence of the fix — the test is.

**Commit.** `951e627`, pushed to `main`; CI run 41551366531 is **success** on `big-s1`. Gate: build, test (48 + 4), serde (55 + 4), mixer (52 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"` and would compile the crate's third predecessor, so it was not run; no public type, field or message
moved.

**Next 5.**

1. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing a `Refusal` payload wants a decision file;
   the log line could be fixed without one, and probably should be.
2. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`. One line to remove in the
   next minor.
3. **A crossfade that a second change interrupts** replaces the target and the fade rather than queueing. Whether a
   game wants queueing, and from which gain it should start, is a design question the log now names.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.
5. **`Rng::next_f32` deliberately discards the low bits** of a xorshift64\* output (the weak ones). A sentence in
   `model.rs` stops the next reader re-deriving it.

**Baseline note.** Nothing was red, and the suite had 44 tests; the fade defect was invisible to it because no test
ever wrote a non-zero `fade_s` and then read the layer before it settled.

---

## Iteration 3 — 2026-11-02 — the seed a project writes, and the hold a stinger promises

**Change.**

- **`src/play.rs`, `src/lib.rs` — `SirenSettings::seed` is the stream.** `SirenChoices` was built once from a crate
  constant and never rebuilt, so a project's `seed:` reached nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. A new `play::seed_choices` system in `SirenSystems::Choose` replaces the stream —
  and every variant history with it — when the mix table's seed changes. It runs before the music starts a layer, so
  a score started on the same frame draws from the new stream too.
- **`src/music.rs` — a stinger's `duck_s` is a hold.** `play_stingers` passed it as `DuckClock::manual`'s *fade*
  argument with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired. It now holds for
  `duck_s` and releases over `DEFAULT_FADE_S` (0.12 s), the release every stop and every steal already uses.
- `examples/render.rs` — a **bus-gain plot** beside the stinger envelope, with both readings of `duck_s` in one run;
  `tests/play.rs` and `tests/music.rs` — five tests, two of which fail on the unfixed tree;
  `docs/loop/needs-decision/stinger-release.md`; `cues.yaml` case 17 rewritten and case 22 added; the cues and music
  guides, the CHANGELOG and a README line.

**Why.** Both are the loop's first-ranked class — wrong output, silent, from a number a person wrote whose sentence
said something else. Case 17 existed to prove reproducibility and could not, because a project could not choose a
stream; the stinger's `duck_s` was documented as a hold and passed as a fade, so the music swelled back underneath the
punctuation mark. Neither needed a new model: `active_duck_db` already had the shape, and the seed field already
existed.

**Evidence.** `runs/2026-11-02-03/`: `eval.md` (baseline green; the four next-5 items checked against the code — one of
iteration 2's was **wrong**, case 9 *does* have tests), `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/` with both render trees, the stinger envelope plot and its zoom, and the cue spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| `seed: 7` vs `seed: 99`, 8 plays | **identical** (variant, gain, pitch) | different from the first play |
| `seed: 7` twice, two processes | identical | identical |
| `Music` halfway through a 0.6 s, −9 dB stinger | **−5.01 dB** | **−8.99 dB** |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.075 / 0.326 / 1.090 µs | 0.023 / 0.135 / 0.508 µs — the machine's load fell 4× between runs; neither fix is on this path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot** (no
legend; the caption reporting the axis top as the peak; the "duck fires" label on the axis, crossing the release
curve) and one in my reading of it — the blue shelf looked like a slope at 900 px until the emitted polyline was
checked frame by frame: frames 39–48 are exactly −9.00 dB. All fixed; the zoom file shows the shelf as a shelf.

**Traps for the command.** (1) An evidence harness that hard-codes the *fixed* shape cannot show the old one — print
**both readings** of a changed argument in one run and let the numbers disagree, rather than rendering "before" from a
reverted tree that the harness does not read. (2) A plot's y-scale must include the cap/depth or a flat shelf sits on
the axis and reads as a curve; check the polyline's y values, not the thumbnail. (3) A `find ~ -iname …` for a paper
takes minutes on this disk — the corpus is on `big`, not here. (4) The home-still embedder was GPU-blocked for all 23
attempts of this iteration (`gpu busy: 4222 MB free < 5000 MB required`); `research.md` records that rather than
pretending a source was read.

**Commit.** `f4fca21`, pushed to `main`; CI run 41549988083 is **success** on `big-s1`. Gate: build, test (44 + 4), serde (51 + 4), mixer (48 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test was not run:
`bevy_cauldron` still pins `bevy_siren` at `rev = "7029b20"`, so it would compile this iteration's *predecessor* and
prove nothing; no public type, field or message moved this time either.

**Next 5.**

1. **`MusicLayers::fade_s` is stored in `Pending` and never read.** Every layer move eases at `MIX_TAU_S` (50 ms), so
   an authored 800 ms crossfade is silently 50 ms. Internal only — carry the fade into `LiveLayer`, or set the easing
   time constant from it — so no decision file is needed.
2. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit; the message shape change wants a decision file.
3. **`needs-decision/remove-completes.md`** — the public `Completes` component cannot ever be true. One line to remove
   it in the next minor, if the owner agrees.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release, shipped this iteration at the crate's
   `DEFAULT_FADE_S`. The field is additive; the default keeps behaviour identical.
5. **`Rng::next_f32` uses the top 24 bits of a xorshift64\* output.** Fine for a variant pick; worth a look before
   anyone uses `SirenChoices::rng` for anything wider, because the low bits of xorshift64\* are the weak ones and this
   implementation discards them deliberately — a sentence in `model.rs` would stop the next reader re-deriving it.

**Baseline note.** Nothing was red at the start, and the suite had 44 tests: the two fixed defects were both invisible
to it, one because no test had ever written a seed and the other because the stinger test read the duck only on the
frame it fired.

---

## Iteration 2 — 2026-11-02 — a voice that ends frees its slot, on both backends

**Change.**

- **`src/mix.rs` — `retire_voices` asks the backend that owns playback.** On the `mixer` backend there is no
  `AudioSink`, and nothing ever set `Completes`, so a `Mode::Once` voice was never retired: it kept its place in the
  voice budget for the life of the process. The system now asks `AudioOutput::finished` — the query the trait already
  declared and `MixerBackend` already implemented, with **zero callers before this commit**.
- **`src/mix.rs` — `update_mix` writes a `MixerVoice`'s own gain.** It used to `continue` past a mixer voice entirely,
  so `Applied` was never written: a music layer moved on the bar stayed at its start gain, and a stop fade began at
  silence and despawned the voice on its first frame — a cut where the crate promises a 120 ms fade. One writer still
  owns both numbers; Bevy's sink gets the whole chain, the mixer's voice stage gets the voice's own gain.
- `examples/budget.rs` (`--features mixer`) — the evidence harness: N one-shots against a cap, a scripted output whose
  voices end after a set life, writing per-frame CSV, a text summary and an SVG plot.
- `tests/mixer.rs` — four tests behind the feature, one of which fails on the unfixed tree.
- `docs/loop/cues.yaml` cases 20 and 21, `docs/guide/buses.md` (both promises made exact),
  `docs/loop/needs-decision/remove-completes.md`, a `CHANGELOG` "Fixed" pair, and a README line.

**Why.** This is the loop's most valuable class of bug — a refusal that should not happen, silent, and in the
backend whose whole point is that a bus is one number. Gregory §14.5.8.4 (*Instance Limiting*): a sound engine
"supports only a fixed number of simultaneous voices … so we must use them wisely", and a voice that stops *"frees
its resources for use by other voices"*. A voice that has produced its last sample and keeps its slot is exactly that
leak. The second defect fell out of the first test: the mixer voice's gain was never written at all.

**Evidence.** `runs/2026-11-02-02/`: `eval.md` (baseline green and why that was the problem; the case table, with
cases 9 and 17 still red), `research.md` (Gregory's lines, `rodio`'s `Player::empty`/`MixerSource`), `plan.md`,
`perf.md`, `review.md`, and `evidence/` with 46 WAVs/text from `examples/render` and four budget runs.

**Measured.**

| scenario | before | after |
|---|---|---|
| 40 one-shots, cap 32, stealable | 40 played, peak **40** live, **0** `ended`, last frame 32 live | 40 played, peak **3**, **40** `ended`, last frame 0 live |
| 6 one-shots, cap 4, priority 180 (unstealable) | **4** played, **2** `VoiceLimit`, **0** `ended`, 4 live forever | **6** played, **0** refused, **6** `ended` |
| a layer moved to −3 dB on the mixer | `Applied` never written; layer stayed at its start gain | `Applied { db: -3.0 }` |
| rendered audio | — | `render.txt` and all 17 WAVs **byte-identical**; cue −24.4 LUFS / −12.1 dBFS, `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.396 µs | 0.065 / 0.332 / 1.233 µs (inside the noise; `perfcheck` does not drive the changed code) |

**The stealable row is the one that matters.** With a stealable cue the leak is *invisible*: every play is answered
`Played`, because the permanently-full budget steals each time — 32 footsteps, then each new one cuts off a footstep
still sounding. The lockout row is what makes it a refusal a reader can see.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found four defects **in my own plot** (a
missing legend entry drawn outside the viewBox, the caption reporting the axis top as the peak, a cap label colliding
with the axis, and a blank raster because CSS variables do not resolve in a rasteriser); all four were fixed before
the verdict.

**Traps for the command.** (1) An evidence plot whose legend is anchored from the right edge silently loses an entry
when the two labels together overflow; anchor from the left and check the `x` extents. (2) A `--features mixer` test
that wants a started voice needs a **resolved** `AudioSource` handle, not `Handle::default()`: `start_pending` gives
a `MixerVoice` only once the bytes exist, so a default handle tests the loading state instead. (3) `errno=28` from
`ld` on this Mac is a full disk, not a code error — `/tmp` held 6.6 GB of my own probe build.

**Commit.** `6a62195`, pushed to `main`; CI run 41545567270 is **success** on `big-s1`. Gate: build, test (41 + 4),
serde (48 + 4), mixer (45 + 4), clippy, fmt, doc, examples, and the README quick start, all green. The mixer-example
link failed once with `errno=28` while the disk was at 100 % — the loop's documented trap, not a code error — and
passed after my own throwaway crate and the editor's build directory were removed.

**The Cauldron was not re-verified, and cannot be by this run.** `bevy_cauldron` pins `bevy_siren` at
`rev = "7029b20"`, so `audio_panel_reads_the_crates_file` compiles the *old* crate: it would prove nothing about this
change. It was attempted anyway and died of `No space left on device` during the 8.3 GB build. The public API did not
move (no type, field or message changed), so the panel's four reads — `SirenMix::db`, `ducked`, `voices_on`, the
`SirenLog` tail — keep their shapes; what changes for the panel is the number in `voices_on`, which is the fix. Its
next pin should move to this commit.

**Next 5.**

1. **Case 9 has no test.** An `UNSTEALABLE_PRIORITY` voice that must refuse rather than be stolen is the rule's other
   half, and the budget's evidence now shows a refusal path worth pinning. It needs a full bus and a dialogue cue.
2. **Case 17 pins the RNG, not the stream.** A determinism test over a whole `Played` sequence (two identical runs,
   equal field for field) is the case's own observation.
3. **`MusicLayers::fade_s` and `Stinger::duck_s` are stored and never read.** A crossfade a project asks for happens
   at `MIX_TAU_S` (50 ms) instead; the field's default is a lie about the shape of the fade.
4. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing the message shape wants a decision file.
5. **`Completes` is public and can never be true.** `needs-decision/remove-completes.md` has the options; the fix
   landed without it, so it is safe to remove in the next minor.

**Baseline note.** Nothing was red at the start. The suite was green with the `mixer` bug present because no test gave a
voice a `MixerVoice` and then let its sample end — which is why the first two items above are about tests rather than
features.

---

## Iteration 1 — 2026-11-02 — the loop's eyes: a render harness and a case table

**Change.** The loop now has a way to judge audio it cannot hear, and none of it touches `src/`:

- `examples/render.rs` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  22.05 kHz mono WAVs plus a `render.txt` of numbers, with no `AudioPlugin` and no `SirenPlugin`: it drives
  `bevy_siren::model`, where every decision's arithmetic lives.
- `examples/perfcheck.rs` — µs per frame and per voice at 8, 64 and 256 voices, an upper bound on `update_mix`.
- `docs/loop/cues.yaml` — 19 fixed cases, each naming its behaviour, its observation and its green condition, with the
  source for every number. The case table is the contract a run is judged against.
- `docs/loop/` — `log.md` (moved from `docs/loop-log.md`), `runs/README.md` and `needs-decision/README.md` describing
  the shape of a run and of a question the loop will not decide itself.
- `.omp/skills/build-loop/SKILL.md` — the loop, written to this brief as a **skill** (the model reads it with
  `skill://build-loop`), with this machine's measured facts in it.

**Why.** The loop's brief demands spectrograms, loudness numbers and panel screenshots before anything ships. The crate
had a panel and a panel is not an ear; nothing could render a cue to a file that `ffmpeg` could measure.

**Evidence.** `runs/2026-11-02-01/`: `eval.md` (the baseline, the case table, loudness), `research.md` (BS.1770-4,
Gregory, the panner), `plan.md`, `perf.md`, `review.md`, and `evidence/` with 23 WAVs and text, four spectrograms and
waveforms, and the duck's gain plot as a line.

**Measured.** 256 voices cost 1.253 µs a frame — 0.0075 % of a 60 fps frame. A cue at −6 dB authors −6.00 dB and reads
−24.4 LUFS (looped past the 0.4 s gating block) at −12.1 dBFS true peak. The duck leaves 0 dB at 1.00 s, reaches
−7.96 dB, and recovers by 1.97 s against a 0.25 s hold and a 0.6 s fade.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT — with one caveat written down: `voices-8.wav` peaks at
3.419 because the harness sums without the voice budget, which is case 19's and the budget's question.

**Traps recorded in the command.** A bus gain is a step function between frames, so one sample per frame is a 4 ms
file; `active_duck_db` takes the time *since the duck fired*, not the score's clock, or the duck never recovers;
`showwavespic` on a gain curve draws an unreadable blob, so a bus plot is a line; and `ebur128` reads −70 LUFS for any
clip shorter than one gating block, whatever its level.

**Commit.** `4e99c64`. Gate: build, test (41 + 4), serde (48 + 4), mixer (41 + 4), clippy, fmt, doc, examples — all
green; nothing was red at the baseline.

**Next 5.**

1. **Case 19:** teach `examples/render` to model the voice budget, or move the case into `perf.md`, so a mix's evidence
   says how many voices it holds.
2. `tests/cues.rs` automating cases 1–4 and 17, so `cues.yaml` and the suite cannot drift.
3. The Cauldron's audio panel should surface the last refusal — a refused audition is a log line today, not a panel row.
4. Occlusion and a low-pass on the `mixer` backend (backlog, iteration 0).
5. Loudness targets per bus: BS.1770-4 is the measure, the target is a `needs-decision/` question.

---

## Iteration 0 — 2026-11-02 — repository setup

**What shipped.** The crate, whole:

- `src/model.rs` — engine-free maths: dB ↔ gain with one silence floor, a linear sum for simultaneous sounds and a
  dB chain for gains in series (a silent stage silences the chain), semitone ratios with a clamp, three distance
  laws and the backend panner's own law divided back out, a duck envelope, an exponential approach, the
  next musical boundary from tempo, a seeded xorshift, a no-repeat variant picker, and voice stealing by lowest
  priority then oldest.
- `src/cue.rs` — `CueTable`, `Cue`, `SoundSlot`, `PitchRange`, `Repeat`, `Mode`, `Attenuation`, and `problems()` in
  sentences. `from_ron`/`to_ron` behind `serde`.
- `src/mix.rs` — `SirenSettings`, `BusLevel`, `Duck`, `DuckClock`, `SirenMix`, `Stopping`, `Applied`,
  `UNSTEALABLE_PRIORITY`; the systems `update_mix`, `apply_stopping`, `retire_voices`, `stop_requests`.
- `src/play.rs` — `Play` → exactly one `Played` or `Refused`; `Voice`, `Compensation`, `Samples`, `MixerVoice`,
  `Completes`, `StopVoice`, `StopCue`, `SirenChoices`; the three caps checked in the order a person asks them.
- `src/playback.rs` — `start_pending`: the one place a voice starts on the crate's own mixer once its bytes load.
- `src/music.rs` — `MusicScore`, `MusicLayer`, `MusicLayers` with `When::{Now, NextBeat, NextBar}`, `Stinger`,
  `MusicState`; layers are voices started through `Play`, so the music has no second spawn path.
- `src/sounds.rs`, `src/log.rs`, `src/output.rs`, `src/mixer/mod.rs` — the key map, the 256-entry log, the
  `AudioOutput` trait and its `rodio` summing graph.
- `src/lib.rs` — `SirenPlugin`, `MusicPlugin`, `SirenPlugins`, `SirenSystems::{Choose, Play, Mix}`.

**Verified.**

| Check | Result |
|---|---|
| `cargo build` (the trimmed feature list a consumer gets) | green |
| `cargo test` | 49 tests, green |
| `cargo test --features serde` | 49+ tests, green |
| `cargo test --features mixer` | 49 tests, green |
| `cargo test --all-features --doc` | green |
| `cargo test --features readme-example --test all readme::` | green (README quick start == crate-root doctest) |
| `cargo build --examples` | green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo run --example music` | transitions at t = 0.00, 4.80 and 7.20 s — every one a multiple of the 2.4 s bar, `transitions on the bar: yes` |

**Captures.** `docs/media/mixer.png` and `docs/media/mixer.gif`, from `examples/mixer` at
`SIREN_FRAMES=240 SIREN_KEYS=steal,layer,duck SIREN_KEY_EVERY=25`, release build, `mean_ms 21.3` (240 frames). The
still reads: Music −9.6 dB with `duck −8.0` and 2 of 12 voices, Effects −3.8 dB with `duck −4.0` and 1 of 2, the cue
table at rest, and a log whose ten lines are the dialogue play, its two duck lines, a steal and its `stolen by`
line. **The captures earned their keep three times**: the first showed a missing glyph (an em dash is not in the
bundled font) and the cue list printed three times (one `Kind::Cues` shared by three nodes); the second showed a
bus at two of two whose steal had already expired; the third showed the music layer at −80 dB because the layer's
level and its cue's trim were both −4 dB. Each was a defect in the example, not in the crate, and each was fixed
before the image shipped.

**One real defect the tests then found.** The capture's steal step asked for three voices in one frame, and all
three played on a bus capped at two: the budget took its snapshot of live voices before the loop, so the frame's
own requests could not see each other. `play_requests` now keeps an owned `Live` list per request, drops what it
stopped and pushes what it started, and
`tests/mix.rs::three_requests_in_one_frame_fill_a_cap_of_two_with_no_third` pins it.

**Cauldron wiring (S-01).** `bevy_grimoire`'s `bevy_cauldron` hosts this crate behind an optional `bevy_siren`
feature, landed as `aea2456` and merged in `d6c6579`, pinned at `7029b20`; `src/audio_panel.rs` reads `<project>/family/bevy_siren.ron` into the crate's own
`SirenSettings` and `CueTable`, draws the bus rows from `SirenMix` and the log tail from `SirenLog`, and auditions the
selected cue with `Play::cue`. `bevy_siren#1` (S-01) and `bevy_grimoire#274` are the two records the two-way street
asks for, both filed in this pass.

Verified on the merged editor tree: `cargo test -p bevy_cauldron --features bevy_siren --test editor` → 1144 passed,
4 failed, and the four are `a_person_finds_and_runs_a_command_in_the_palette`, which fail identically with the
feature off (checked: 0 passed, 4 failed) — they predate this work. `cargo clippy -p bevy_cauldron --features
bevy_siren --all-targets` is clean, and `boot_headless` opens the shipped project on both feature sets: 12 frames,
2048 entities, no panic. The four panel tests read `Played`/`Refused` through a reader system: draining the doubled
message buffer after several updates lost one in the full parallel run, and a host reads every frame, so the test
does too.

**Decided (assumed, owner may override).**

- **Two backends, not one.** `bevy_audio` by default and our own `rodio` graph behind `mixer`, because a bus's gain
  as one number per bus per frame is the point of a bus, and Bevy's path cannot express it — but Bevy's path is the
  one that pans a moving emitter, so it stays the default.
- **The panner's distance law is divided back out.** rodio's `Spatial` multiplies each ear by `min(1, 1/d²)` as well
  as panning, so the crate's own attenuation would be applied twice. `model::backend_spatial_gain` mirrors that
  formula; `tests/model.rs::the_backend_spatial_gain_is_the_panner_distance_law` pins it.
- **`UNSTEALABLE_PRIORITY` is 180.** Music (200), stingers (210) and dialogue (180) keep their voices when effects
  fill the budget. A number, not a flag, so a project can position a cue between them.
- **The default duck is Gregory's pair.** Dialogue lowers music 8 dB and effects 4 dB, hold 0.25 s, fade 0.6 / 0.4 s.
- **`rust-toolchain.toml` pins 1.99.0.** `bevy 0.20` and this crate require 1.97.1; the machine's `stable` was 1.94,
  which cargo refuses. 1.99.0 is installed here and CI's `dtolnay/rust-toolchain@stable` is newer still.

**Backlog, ranked.**

1. **Occlusion and a low-pass on the `mixer` backend.** A wall between the listener and a sound should muffle it, and
   the mixer has no filter in its graph today. Needs a rodio filter source per voice and a ray-cast hook the game
   supplies — the crate must not own physics.
2. **A dialogue queue with subtitles.** `Dialogue` voices are unstealable but unordered: two lines at once play over
   each other. A queue with a `DialogueLine { speaker, text, cue }` and an accessibility subtitle stream is what a
   game actually needs next.
3. **Per-voice reverb zones.** A zone is data (a box and a wet mix); a voice inside it gets a send. The mixer path can
   do this properly; Bevy's path cannot, which is a reason for the `mixer` feature to grow.
4. **`SirenProbe`.** A way to ask "what would this cue play, and at what gain" without playing it, so the Cauldron's
   panel can preview a decision on hover instead of on click. Filed here as an `S-NN` when the editor needs it.
5. **A moving emitter's pan on the `mixer` backend.** rodio's `SpatialPlayer` can be re-positioned; a mixer-owned voice
   cannot today, which is the one thing Bevy's path does that ours does not.

**Release: published.** `cargo publish` from `big` (whose `~/.cargo/credentials.toml` carries the `[registry]` token;
this Mac has none, and the first attempt here was refused for that reason) uploaded `bevy_siren 0.1.0`. The registry
answers 200 for `https://crates.io/api/v1/crates/bevy_siren` with `max_version 0.1.0` and `Ladvien` as owner, and the
tag `v0.1.0` is pushed at `8e14231`. `docs.rs/bevy_siren/0.1.0/bevy_siren/` answers 200 — docs.rs had built it within a
few minutes of the upload, and the bare crate URL lags that by one more sync. The README's own image URLs answer 200.

**CI.** Two things were wrong, and neither was the code. Every sibling repo has a self-hosted runner registered and
this new one had none, so GitHub had nowhere to run; the runner is now deployed (`bevy-coven/bevy_siren 1` in
`home-cloud/gh-runners/repos.txt`, `deploy.sh`, `big-s1 online`). And `run: cargo test … readme::` is **not valid
YAML** — unquoted, `readme::` parses as a mapping and GitHub rejects the whole workflow, which is why every run still
failed in 0 seconds with no job even with a runner present. Quoted, it is a string. Run 41527805041 is the first real
one: **both jobs green on `big-s1`, 4m29s** (`fmt + clippy + doc`, then `build (standalone) + test`).
