# Iteration 123 — review

**Item**: `needs-decision/log-cursor.md` option **4** — **an authoring refusal is printed once on the console**, filterable
by target, and a budget refusal never is. (The release that was the top item needs the owner's explicit argument and is
not this iteration's.)

## Scorecard — the wrong ones first

| | what was said | what happened |
|---|---|---|
| **P5** | a hundred plays of one unknown cue in one frame print **one** line | **Wrong at the first cut**: the first run of the change printed **a hundred**. The probe caught it (`evidence/after/refusals-after.txt` keeps the fixed run; the per-frame set was added because of it) and `tests/console.rs` now holds both directions. A rule in a plan is not a rule in the code |
| **M3** | the line must be keyed by cue **and reason** | **The mutation was green and no test could be written for it**: `answer` returns the first reason it meets, so no cue is refused for two reasons in one frame. The key stays (a bus checked first would make it reachable) and the test file **says it is unheld** instead of claiming a test |
| **P7** | the line's sentence equals the log's `Refused` reason | Held, and the test had to read the log through `Entry::line` and split the cue off — a first form compared two independently built sentences and would have passed while they drifted |
| P1 – P4, P6, P8, P9 | silent before; four authoring classes print, once each per cue and reason per frame; `VoiceLimit` and `SpatiallyMute` silent; no subscriber safe; the quick start's console unchanged when its cues are right; no public surface | Held: the before table is ten zeros; the after table is 1 / 1 / 8 / 1 / 1 / 1 / 0 / 0; the four tests; the real run (`evidence/after/typo-on-the-real-console.txt`) |

## Pass 1 — function

*Critique.* (1) **The class split is where the refusals are defined**, not in a rule far from them: `Refusal::is_authoring`
sits beside the variants and names the reason for each (`VoiceLimit` is *"the budget doing what a project asked for"*,
`SpatiallyMute` is *"an architectural state that can precede the listener by a frame"*). (2) **The sentence is the log's
own** (`refusal_sentence`), prefixed with the cue — one sentence, two places, held by a test. (3) **`target: "bevy_siren"`**
and `warn!`: filterable by target and level through `RUST_LOG` or `LogPlugin`, silent with no subscriber (held by a test
and by the 600 existing tests, none of which installs one). (4) **No public surface**: `is_authoring` and `kind` are
`pub(crate)`; no field, method, message or dependency. (5) **`src/log.rs`'s header** said *"Nothing here is a `println!`.
A crate that prints is a crate a game cannot silence"* — that is now half true and is rewritten: the crate prints an
authoring refusal, filterably, and the log itself is still written in silence.
(6) **What is deliberately not printed**: a budget refusal (a cap doing its job, `cues.yaml` 7–9, 21) and a spatial mute.
*Verdict:* **GREAT.**

## Pass 2 — the number

*Critique.* (1) **Before and after are one program** (`evidence/{before,after}/refusals-probe.rs.txt`), ten refusal
classes, the same capture: **0** lines everywhere before, and the table after — 0 healthy, 1 per authoring class, 1 for a
hundred plays in a frame, 2 for two cues, 8 for eight frames, 0 for the budget, 0 for a spatial mute. (2) **The first
run of the change printed 100 for 100 plays** — the probe's hundred-row is why the per-frame set exists; the fixed run is
kept and the finding is in the evidence file. (3) **On the real device** (`evidence/after/typo-on-the-real-console.txt`): the
README's quick start with one letter wrong, built and run on this Mac's window and output device, prints
`WARN bevy_siren: refused \`footstep.grss\`: no cue of that name` **once a second**, with no game code for it — the
example has no `report` at all. (4) **The sound did not change**: 21 of 21 rendered WAVs byte-identical to iteration
122's, every refusal text identical, `render.txt` identical but for the `variants` lines, `mix.wav` −21.5 LUFS / −7.5 dBFS,
its spectrogram the same picture. (5) **`perfcheck`** 0.058 – 0.062 / 0.325 – 0.358 / 1.209 – 1.304 µs, inside iteration
122's band; a frame with no refusal pays an empty `HashSet`.
*Verdict:* **GREAT**, after the per-frame set; **NOT YET** at the first cut, which printed a hundred lines.

## Pass 3 — fresh eyes

*Critique.* (1) **A reader who has never opened the guide, with one letter wrong**, now sees the cue's name and *"no cue
of that name"* on their own console — the case the whole item exists for, shown on the real device. (2) **A game that
wants silence** has `RUST_LOG=bevy_siren=off`, and the guide says so. (3) **The guide's refusal table** gains the column
that matters: which refusals print, and why the other two do not. (4) **The picture** is a terminal transcript from the
real run and the probe's before-and-after table — not a composed image; the media entry names the command. (5) **The
existing quick-start test** filters the console by its own module target, so its "the console is the crate's log" assertion
is about the block's `report`, not the crate's line — checked by running it, not assumed. (6) **`docs/loop/needs-decision/log-cursor.md`**
gains the decision (option 4, built) with what is left: nothing but the release of the two stream changes.
*Verdict:* **GREAT.**

## Catches this iteration

1. **The first cut printed a hundred lines for a hundred plays** — the probe's own row caught it; the per-frame set was
   the fix, and the test now holds both directions.
2. **A test for a state the code cannot reach** (M3): the mutation was green, and the honest response was to say so in
   the file rather than invent a test.
3. **The budget test's first form found no `Refused` at all** — 40 plays of a cue with `max_instances: 2` are *stolen*
   from, not refused, so the test proved nothing; a cue at `UNSTEALABLE_PRIORITY` against a two-voice table is a real
   `VoiceLimit` (`cues.yaml` case 9's rule).
4. **The probe was silent in its first two runs** for a reason worth keeping: `with_default` installs a subscriber on
   **this** thread, so `Update` has to be forced single-threaded (`SingleThreadedExecutor`) — the pattern
   `tests/quick_start.rs` already documents, and the one `AGENTS.md` names.

## Tool discipline

Reads and edits with `read`, `grep`, `glob`, `edit`; bash ran programs (`cargo`, `ffmpeg`, `md5`, `cmp`, `df`, `git`), and
`perl -0pi` **only** for mutations, each reverted and checked by hash (`src/play.rs` 485b503e… before and after). One slip:
a `python3` rewrite of a markdown table's column widths, where an `edit` would have done.
