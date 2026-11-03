# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Entries are **newest first**; iteration 0 is the repository's creation. Rules and traps are in
`AGENTS.md`; the design rules and their sources are in `../research/siren.md`; the fixed cases every iteration runs are
in [`cues.yaml`](cues.yaml); each iteration's own research, plan, evidence and three-pass review are in
[`runs/YYYY-MM-DD-NN/`](runs/README.md), and a question the loop will not decide itself goes to
[`needs-decision/`](needs-decision/README.md).

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
