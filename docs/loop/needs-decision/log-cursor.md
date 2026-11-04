# `SirenLog`: a cursor to read the log with

*Iteration 120, 2026-11-04. Two public methods are added; built under the standing "recommend, record, proceed" rule (`assumed-decision`).*

## What changed in the public surface

- **Added**: `SirenLog::pushed(&self) -> u64` — how many entries have ever been pushed, a count that never goes down — and `SirenLog::since(&self, mark: u64) -> impl Iterator<Item = &Entry>` — the entries pushed at or after `mark` that the ring still holds, oldest first.
- **Unchanged**: `push`, `recent`, `iter`, `len`, `is_empty`, `clear`, `CAPACITY`, `Entry` and `Entry::line`; `SirenLog` is still a `Resource` and `Clone + Debug`. Its fields are private, so the new representation (`VecDeque<(u64, Entry)>` and a counter) is not visible: **+8 bytes an entry**, 2 KiB for a full ring.
- **Behaviour**: none changed. `clear` empties the ring and **leaves** `pushed`, so a mark taken before a clear still reads what came after it.

## Why

The README's Quick start has to *show* what the crate decided — `docs/loop/runs/2026-11-04-120/` found that it printed nothing and wrote a second, mistimed entry into the log instead. Printing the log means printing **each entry once**, and the log is a 256-entry ring that drops its **oldest routine entry** (not its oldest) when it is full. There was no way to ask *"what is new since I last looked"*.

## Options

| # | option | cost to a consumer | verdict |
|---|---|---|---|
| 1 | **`pushed()` and `since(mark)`**, each entry carrying the number it was pushed as (this change) | two additive methods; a game keeps one `u64` | **chosen**: exact, tiny, the game owns its mark |
| 2 | A `Logged` message stream — a `Message<Entry>` written beside every `push` | none to read it | no: every decision is cloned (four `String`s) into a message at frame rate whether or not anyone listens, and the log's rule for which entry leaves a full ring would have to be held in two places. A game that wants the stream can build it on option 1 in three lines |
| 3 | Nothing: a game uses `at_s` as its cursor | none | no: every entry of one frame shares one time, so a strict `>` loses entries pushed after the reader ran in that frame and `>=` prints them twice; and the other obvious cursor, `seen = log.len()`, **printed 256 of 600 entries and went quiet for good** (`evidence/before/len-cursor-stalls.txt`) |
| 4 | **The crate prints** — a `warn!` once per cue and reason for an *authoring* refusal (`UnknownCue`, `NoSounds`, `MissingSound`, `UnknownBus`), filterable by target | a line on every game's console | **built in iteration 123** (`runs/2026-11-04-123/`): `src/log.rs:7` says *"A crate that prints is a crate a game cannot silence"* — a `tracing` event **can** be silenced (`RUST_LOG=bevy_siren=off`, a `LogPlugin` filter), which is what settles it — and `warn!` is a `tracing` event, so the sentence becomes *the log is written in silence; the console is told about an authoring mistake*. A `VoiceLimit` never prints (normal operation, `cues.yaml` 7–9 and 21) and a `SpatiallyMute` never prints (architectural); no setting was added — `RUST_LOG` is the setting a Rust project already has |

**Option 4 was built in iteration 123.** Ten refusal classes printed nothing before it; after it, an authoring refusal
prints once per cue and reason per frame on `target: "bevy_siren"`, the budget and a spatial mute never print, and the
line is the log's own sentence (`tests/console.rs`, four tests; the real device's console is
`runs/2026-11-04-123/evidence/after/typo-on-the-real-console.txt`). `is_authoring` is `pub(crate)`: no public surface.

## Edge cases and what is decided

- **A reader that falls more than `CAPACITY` behind loses entries.** `pushed() - mark` larger than what `since` returned says how many; nothing is invented.
- **A mark ahead of the count returns nothing** — a log that was replaced by a shorter one.
- **Exact, not approximate**: the answer goes by push number, not position. With the ring full of refusals and one play held, two further plays evict the play before them and then the one pushed *after* the mark; "the last `pushed() - mark` entries by position" returns `["r254", "c2"]` for `["c2"]` (`tests/model.rs::since_is_exact_when_the_ring_evicts_an_entry_after_the_mark`, the only test a positional form reddens).
- **Overflow**: a `u64` at a million entries a second lasts about 584 000 years.
- **Cost** (release, a full ring, so every push also evicts): a push is **3 – 5 ns** on top of building the entry (about 30 ns); `since` is a binary search and costs **10.5 ns** with nothing new and **11 ns** with two new entries — the quick start's `report` is that, once a frame. `perfcheck` does not push entries and reads inside its band (`perf.md`).

## Recommendation

Option 1, as built; **option 4 was built in iteration 123** with the measurement this file asked for — a healthy game refuses nothing it prints, and the sentence at `src/log.rs` is rewritten to *the log is written in silence; the console is told about an authoring mistake*.
