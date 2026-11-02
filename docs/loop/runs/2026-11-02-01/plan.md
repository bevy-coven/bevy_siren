# plan.md — build the loop's eyes

## The item

Not a crate feature. The loop it is meant to run needs a way to judge audio without an ear, and the crate had none:
`examples/mixer` draws a panel and `examples/music` prints transition times, but nothing renders a cue, a bus or a mix
to a file that can be measured.

## Design

- **`examples/render.rs`** — a headless harness with **no `SirenPlugin` and no `AudioPlugin`**. It drives
  `bevy_siren::model` directly, because that is where every decision's arithmetic lives, and writes 22.05 kHz mono
  16-bit WAVs plus a `render.txt` of numbers. One file per cue variant, one per refusal (a `.txt`, because a refusal is
  a decision), one per bus envelope, a `mix.wav`, and `voices-<n>.wav`.
- **`docs/loop/`** — `log.md` (the old `docs/loop-log.md`, moved so the loop's state is in one directory), `cues.yaml`
  as real YAML with 19 cases, `runs/README.md` and `needs-decision/README.md` describing the shape a run takes.
- **The loop, as a project skill** — `.omp/skills/build-loop/SKILL.md`: the brief's steps in this project's terms, with
  this machine's measured facts in it. A skill and not a command, because it is the model's procedure and one name has
  one home.
  the toolchain pin and Bevy's floor, `ffmpeg` 9 with `ebur128`, and the disk-full failure mode.

## Modules touched

`examples/render.rs` (new), `examples/perfcheck.rs` (new), `docs/loop/*` (new and moved),
`.omp/skills/build-loop/SKILL.md` (new), `README.md` and
`AGENTS.md` (their pointer to the log). No `src/` file changes, so no public API moves — nothing goes to
`needs-decision/`.

## New tuning fields

None. The harness reads `SirenSettings::default()` and the cue table; it introduces no constant of its own except the
synthesis stand-in's frequency and decay, which are a fixture and labelled as such in the file.

## Tests

- The harness is proved by running it and **reading** its output: the duck's line plot, a cue's spectrogram, and the
  LUFS numbers in `eval.md`.
- `cargo build --examples` proves it compiles in the gate.
- The crate's own suite is untouched and stays green.

## Carved off

A `tests/cues.rs` that automates all 19 cases. Five of them (1–4, 17) are already covered by `tests/play.rs` and
`tests/model.rs`, and the rest are the play system's decisions, so the case table names the observation and the test
that proves it rather than duplicating them. Automating the table belongs to the first iteration that changes a case.
