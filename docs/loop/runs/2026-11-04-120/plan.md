# Iteration 120 — plan

## The pick, in one sentence

**The README's Quick start plays nothing, shows nothing from this crate, and warns the reader that two of its own functions are dead — and no test could have said so, because none runs it.** It beats the runner-up (a spatial sound on a moving owner, 119's carry-over) because it is a **wrong-output bug on the one code path every adopter takes first** (rank 1, found by running the block verbatim on a real window and device), it is a **blocker in the sense of the literature** (Uddin and Robillard 2015: incorrectness, incompleteness; six of ten documentation problems made developers abandon an API), and the runner-up is a missing refinement of something that works (the pan already tracks the owner; only the level law is frozen).

## Ranked

| # | item | rank | why here |
|---|---|---|---|
| 1 | **The front door: the block plays, shows why when it cannot, and a test runs it** | 1 (wrong output) + 3 (authoring) | reproduced; four defects, one cause; carried over twice |
| 2 | `.on(owner)` of a spatial cue with no `.at` is placed at the owner; the level law follows a moving owner | 2 | works today, flat; a behaviour decision (`needs-decision`) and a measurement first |
| 3 | The layers of a score start together | 2 | 3 of 100 warm-cache starts of eight stems, one frame apart |
| 4 | A person listens | 5 | needs a person |
| 5 | The waiting items | — | |

## What is carved off

* **The crate printing authoring refusals itself** (`warn!` once per cue and reason, filterable by target). It would make the failure loud for a game that never reads the log, but it is a **design question** — `src/log.rs:7` says *"A crate that prints is a crate a game cannot silence"*, a `VoiceLimit` refusal is *normal operation* under load and must not be printed, and a setting for it is a schema change the editor reads. Recorded in `needs-decision/log-cursor.md` as option C and in the top five; this iteration shows the refusal **in the block**, where a reader sees it, and gives a game the one primitive a printer needs.
* `Refused` carrying the cause (a `Refusal` variant; already weighed in `needs-decision/undecodable-sound.md`).
* The Cauldron's own quick start (a file the editor writes).

## The design

### 1. One file is the block: `examples/quick_start.rs`, with the files it plays

The reader's code, **character for character** the README block and the crate-root doc block:

```rust
use bevy::prelude::*;
use bevy_siren::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SirenPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (walk, report))
        .run();
}

fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
    // A cue with two variants: the picker never plays the same one twice in a row.
    // A sound key is a file's name under `assets/audio/`, extension included.
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01.ogg"), SoundSlot::new("step_02.ogg")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

// A footstep every half second.
fn walk(mut play: MessageWriter<Play>, time: Res<Time>, mut since: Local<f32>) {
    *since += time.delta_secs();
    if *since >= 0.5 {
        *since -= 0.5;
        play.write(Play::cue("footstep.grass"));
    }
}

// What the crate decided, once each, in its own words: which recording played, or why nothing did.
fn report(log: Res<SirenLog>, mut seen: Local<u64>) {
    for entry in log.since(*seen) {
        info!("{}", entry.line());
    }
    *seen = log.pushed();
}
```

* `assets/audio/step_01.ogg`, `step_02.ogg` — two synthesized footsteps (brown noise, filtered and enveloped; the parameters are in `research.md`), **made for this repository, CC0**, with the `ffmpeg` commands in `assets/README.md`. `Cargo.toml`'s `include` already leaves `assets/` and `examples/` out of the published package: a crate user supplies their own files, a clone has these.
* **Why the first step is not at t = 0:** `walk` accumulates, so the first step is at 0.5 s, when both files have loaded; a play whose load is in flight is not refused, but the first sound a reader hears should not depend on it.
* **`report` replaces `hear`.** `hear` copied what the crate already wrote into the crate's own log with `at_s: 0.0`; `report` *reads* the log and prints it, which is the thing the crate's whole *"why did that happen"* pitch is.

### 2. One additive primitive: `SirenLog::pushed()` and `SirenLog::since(mark)`

`report` has to print each entry **once**, and the log is a 256-entry ring that drops its **oldest routine entry** when full (not always the oldest entry). The obvious cursor — a `Local<usize>` over `len()` — stops printing once the ring is full (P3). `at_s` is not a cursor either: every entry of a frame shares one time.

* `entries: VecDeque<(u64, Entry)>` — each entry carries the number it was pushed as; `pushed: u64` counts every push, **never goes down, and `clear` leaves it**.
* `pub fn pushed(&self) -> u64` — how many entries have ever been pushed.
* `pub fn since(&self, mark: u64) -> impl Iterator<Item = &Entry>` — the entries pushed **at or after** `mark`, oldest first, **exactly** (the sequence number, not a position, so an eviction in the middle of the ring cannot make a read repeat an entry or skip one). Entries the ring has already dropped are gone: `pushed() - mark` larger than what was returned says how many.
* Cost: +8 bytes per entry (2 KiB for a full ring); `since` is a binary search (`partition_point`) and the walk over what is new.
* **Default / citation**: none to cite — a counter, not a rate or a threshold; **design default** for the names (`mark` mirrors a read position). Additive; **`docs/loop/needs-decision/log-cursor.md`** (options: this; a `Logged` message stream; nothing — games use `at_s`; the crate printing). "Recommend, record, proceed."

### 3. The tests

`tests/quick_start.rs` — the block **run**, not compared:

* the example is brought in with `include!("../examples/quick_start.rs")` (so the functions are in scope, private as the reader's are; `main` is dead there and allowed to be);
* an app of `MinimalPlugins`, `AssetPlugin::default()` (**the plugin `DefaultPlugins` adds: `assets/` at the crate root**), `init_asset::<AudioSource>()` and **Bevy's real `AudioLoader`** (what `AudioPlugin` registers), the clock driven by `TimeUpdateStrategy::ManualDuration(100 ms)`, `SirenPlugin`, the `Output` removed so **Bevy's path owns every voice in every build**, and `Update` forced single-threaded so a thread-local capture sees every system;
* `setup`, then updates (real sleeps while the loader's threads work, bounded) until both files are loaded, then four seconds of frames;
* **the console is captured** — a `tracing` `Layer` through `bevy::log::tracing_subscriber` (no new dependency) collecting the `message` of each event — and the assertions are on **those lines**:
  1. `the_quick_start_plays_both_recordings_and_the_console_says_so` — ≥ 6 `played` lines, naming `step_01.ogg` and `step_02.ogg` and alternating; no `refused` line; the same ≥ 6 `Played` messages;
  2. `a_key_without_its_extension_is_named_on_the_console` — the old keys through the same `report`: a line with `failed to load` and `assets/audio/step_01`, and no `Played` that sounds;
  3. `the_quick_start_adds_every_system_it_defines` — read the file: every `fn` but `main` is named in `main`'s body (the compiler's `never used` warning, as a failure);
  4. `the_crate_root_doc_is_the_example` — the `no_run` block in `src/lib.rs`, stripped of its `//! `, equals the example (un-gated: a plain `cargo test` holds it);
* `tests/readme.rs` (still behind `readme-example`, so the matrix's own command runs it) compares the **README block to the example**, instead of to the doctest — one comparison to the file that is run;
* `tests/model.rs` — the cursor: (a) a reader with a mark reads each of 1 000 entries once across the ring's wrap, at several read cadences; (b) exact when a routine entry pushed after the mark is evicted from a ring otherwise full of refusals; (c) `clear` keeps the count and a mark from the future reads nothing; (d) a mark older than the ring gets what is left and `pushed() - mark` says how many were lost.

### 4. Docs

README (the block, the sentence *where the files go*, `cargo run --example quick_start`, and a *Try it* line); `src/lib.rs` (the block, `SirenLog::{pushed, since}` docs); `docs/guide/cues.md` ("The keys and the files": `load_table`'s convention — *the key is the file's name under the directory, extension included* — and the false sentence at 147–149); `AGENTS.md` (layout, a trap); `CHANGELOG.md`; `docs/loop/cues.yaml` case 41; `docs/media/quick-start.{txt,png}` (the real run's console) and `docs/media/README.md`; `needs-decision/log-cursor.md`; the build-loop skill's lesson (*run the block*).

## Modules touched

`src/log.rs` (the cursor), `src/lib.rs` (the doc block), `examples/quick_start.rs` (new), `assets/` (new), `tests/quick_start.rs` (new), `tests/readme.rs`, `tests/model.rs`, `tests/all.rs` (a `mod` line), docs as above. **No change to a system, a schedule, a message or a schema.**

## How it will be proved

Red first: the four new `quick_start` tests and the cursor tests do not compile on the unchanged tree (`SirenLog::since` does not exist) — that run is kept; and the **behavioural** red is `evidence/before/readme-as-written.txt` (nothing plays; the block's own compiler warning). Then green, then **mutations** (P8): `walk` left out of `main`; a key's extension dropped; the README block edited; the lib.rs block edited; `since` off by one; the sequence number replaced by a position. Then the gate: the render (21 WAVs), `mix.wav` loudness, a spectrogram read, **the two new footsteps' spectrograms, waveforms and loudness**, `perfcheck` ×3, and the real run of `cargo run --example quick_start` on this Mac's window and device with its console kept.

## What happened to the plan

* **The block's local in `walk` was renamed `since` → `elapsed`** after the first full matrix: `*since += …` two lines above `log.since(*seen)` read like one name for two things. It is in all three copies; the matrix was run again.
* **Three cursor tests, not four**: the "`clear` keeps the count", "a mark from the future reads nothing" and "a mark older than the ring gets what is left" cases are one test (`the_count_survives_a_clear_and_a_stale_mark_gets_what_is_left`); the ring-wrap and the eviction-after-the-mark tests are as planned. `quick_start` has the four planned tests; the `unscheduled` check has its own negative fixture (the old block's shape), which the plan did not mention.
* **`crate_root_quick_start()` lives in `tests/quick_start.rs`**, not in `tests/helpers.rs`: once `tests/readme.rs` compared the README to the example instead of to the doctest, only one file read the crate-root block.
* **A first form of the console test indexed `lines[..said.len()]`** and panicked with a slice error when the console printed more than the log held (mutation M7); it now says *"something was printed twice"* first.
* **A temporary scratch test (pass 3)** ran the block's own `walk` and `report` over another asset directory, a three-variant cue and five seeds. It passed — and surfaced a defect that is not this item's (`evidence/after/fresh-eyes-seeds-and-cue-set.txt`): **the first pick of every small seed is the first variant.** Carried to the top of the next iteration.
* **The disk** reached 1 GB free. `cargo clean -p bevy_siren` and then `cargo clean --release -p bevy_siren` (this crate's own artifacts, built in this session; the matrix script's own clean is debug-only) freed 4.6 GiB, and the final matrix runs with `CARGO_INCREMENTAL=0`. The first final matrix failed two large links (`semantic`, all features) with `linking with cc failed` at 2.7 GB free — **[INFERENCE] the disk and not the code**: the same commands linked at 3.5 GB and, with incremental caches off, at 3.7 GB, and the script keeps no line of `ld`'s message to show `errno=28`.
