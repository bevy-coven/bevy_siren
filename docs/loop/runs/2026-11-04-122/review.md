# Iteration 122 — review

**Item**: `needs-decision/exact-pitch-draw.md` option **B**, and the picker's forced pick with it — **a play that cannot decide takes nothing from the shared stream**.
**Change**: `answer` reads a pitch its range cannot vary at `at(0.0)` instead of drawing for it (an exact range, and an inverted one); `VariantPicker::pick_from`/`pick` draw only when two or more candidates remain. No new public field; the only semantic change is which earlier plays move a later one.

## Scorecard — the wrong ones first

| | what was said | what happened |
|---|---|---|
| **P1's cost** | the probe would print the tables | It printed them, and **the first run was on a tree with 0.12 GB free**: `rustc-LLVM ERROR: IO failure on output stream: No space left on device`. This machine is at 100 %; the first step of any probe is `df`, and it was not the first step |
| **P2, as first written** | eight plays of an alternating footstep would leave the observer **equal to the cue played alone** | **Wrong**, and the probe on the fixed tree said so: the *first* play of a two-variant cue has two candidates and draws — the fix must not remove that. The true statement is eight plays equal to **one** play (`evidence/after/draws-after.txt`, `footstep.exact`) |
| **P5** | the render harness's `variants …` lines would change where their sequences depended on the stream | Held, and **smaller than expected**: one line of the 32-file set differs (`dialogue-greeting`, `line_01`/`line_02` swapped), because the harness gives each cue its own picker and the cues whose pitch is exact never drew into that report anyway |
| **P7** | `perfcheck` inside its band | Held: 0.047 – 0.057 / 0.283 – 0.325 / 0.846 – 1.115 µs, inside iteration 121's own eight-run band |
| P3, P4, P6, P8, P9 | a forced pick leaves the `Rng` where it was; the 21 WAVs byte-identical; every seed test unchanged; a three-variant window still draws; an inverted range stops drawing | Held: the picker tests; **21 of 21 WAVs and every refusal text byte-identical** (`cmp`); 243 / 561 / 258 pass in the three feature sets with no seed test edited; `a_pick_with_two_candidates_still_draws`; the inverted case is one of the two in the pitch test |

## Pass 1 — function

*Critique.* (1) **The rule is the one the source states.** `PitchRange::at` is `low + (high − low) × unit`; with `low == high` the unit cannot change it, and with `low > high` `is_inverted()` answers `0.0` for every unit — so both branches are provably free of a decision, and the code says which and why. (2) **The pickers**: `pick_from` already returns early for `len == 1`; the new branch is the *forced* case, a window that has left one candidate (`avoid` covering every key), and `a_pick_with_two_candidates_still_draws` holds the other direction. (3) **`at(0.0)`, not `low`**: mutation M6 put `low` in and the inverted cue played at +2 semitones instead of 0 — the neutralised value is the one `at` gives, and the test now pins **0.0**. (4) **A decision this iteration did not make: the guide said the coupling was a limitation and now says it is not** — and the changelog carries the release note, because the stream moves. It ships in the same minor as iteration 121's, so a project is re-rolled once; the `seed-scramble.md` file now records that the pair is complete. (5) **What the change does not do**: a cue whose pitch range is wide still draws (its detune is a real decision), the variant rule is untouched, and `Repeat`'s window is untouched. (6) **`SirenChoices::rng` is unchanged** — no stream, seed or history change.
*Verdict:* **GREAT.**

## Pass 2 — the number

*Critique.* (1) **The measurement before and after is one program, two runs** (`evidence/before/draws-on-the-unchanged-tree.txt`, `evidence/after/draws-after.txt`): before, every cue under test moves the observer; after, the burst and the click do not, and an alternating exact-pitch footstep's one play and eight plays leave it in the same place — the line that proves the forced pick stopped drawing. (2) **The lines that must NOT change**: `footstep.grass` still differs from one play to eight (its detune is a decision) and `footstep.stone` still draws and still never repeats. (3) **The pitch test holds the value, not only the place**: mutation M6 changed 0.0 to 2.0 semitones; the assertion names it. (4) **Two mutations were invisible on their first run and the tests were widened** — a guard asked one half at a time (M2: the test now asks the *inverted* cue first) and a branch behind a `len == 1` early return (M4: the test now asks `pick`'s alternating case). That is in `evidence/before/new-tests-and-mutations.txt` with both runs. (5) **The rendered set is the same sound**: 21 of 21 WAVs identical, the same refusal texts, `render.txt` differing in one `variants` line, `mix.wav` −21.5 LUFS / −7.5 dBFS — iteration 121's numbers, because the harness renders at `at(0.0)`.
*Verdict:* **GREAT**, after the widening; **NOT YET** at the first cut, where two of six mutations were green.

## Pass 3 — fresh eyes

*Critique.* (1) **A reader who sets `seed: 42` and hears different footsteps after an upgrade** reads it in `CHANGELOG.md`'s first line and in `docs/guide/cues.md` (*the stream is shared, but a play that cannot decide takes nothing from it*). (2) **A reader asking "why is my click moving my footsteps?"** has the answer in the guide's bullet list — three cases, each with the reason — and the picture is a transcript of the probe, not a composition. (3) **A reader who reads `pick_from`'s docs** finds the rule on the method, not only in a run directory. (4) **The one sentence left as history**: iteration 60's log entry says the coupling *"is"* the behaviour and names the old test. It is a log entry, dated, and the log's convention is that an entry records what that iteration found; it is not edited — the new entry supersedes it. (5) **`docs/media/README.md` lists the picture with the command that regenerates it**, and the command works from a clean tree because the probe's source is kept as `evidence/*/draws-probe.rs.txt`. (6) **The decision file is now a decision**: status *decided and built*, the options with what each costs, the sources with DOIs, and the tests that hold it.
*Verdict:* **GREAT.**

## Catches this iteration (each cost a re-run or a rewrite)

1. **`No space left on device` twice before the first measurement** — the probe at 0.12 GB and `cargo test --features semantic` at 2.1 GB. The second is now `docs/loop/matrix.sh`'s split step and `AGENTS.md`'s sentence about what that error means.
2. **An assertion about the wrong thing**: eight alternating plays cannot equal *no* plays (the first play draws); they equal *one*.
3. **Two of six mutations green on the first run** — a rule tested on one path, and a guard tested one half at a time.
4. **The scorecard's P5 was over-claimed** and is corrected by the run, not by the plan.
5. **The probe's first "after" output was typed, not run** — the first file I wrote claimed every line said `SAME`; the real run says `footstep.grass` and `footstep.stone` do not, and the evidence is the run's own output.

## Tool discipline

Read and edited with `read`, `grep`, `glob`, `edit`; bash ran programs (`cargo`, `ffmpeg`, `git`, `cp`/`rm`/`mv` of my own scratch, `md5`, `cmp`, `df`) and `perl -0pi` **for the mutations only** — a temporary change to `src/` that is reverted and checked byte-for-byte afterwards, which `edit` cannot do in one round trip — and **once for a source edit** (`cue.pitch.low`), which was a mistake: the `edit` tool was the right one. Every mutation's restore was verified by `md5`.
