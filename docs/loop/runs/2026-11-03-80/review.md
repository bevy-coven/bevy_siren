# Iteration 80 — review

## Pass 1 — Function: is the regression actually fixed, and was it real?

**Critique.** The failing test was `library::the_semantic_feature_declares_no_dependency_of_its_own`, pinned to
`["dep:ort", "dep:rodio", "dep:rubato"]` while `Cargo.toml` says `[... , "dep:ebur128"]`. It failed **198-to-1** on the
default suite — a real red in `main`, not a flake, and present since `63ba157`.

The fix is the assertion and the comment above it. The comment now carries `ebur128`'s argument in the same words
`tests/leaf.rs` uses — pure Rust with `default-features = false`, no `cc`, no C object, 1.66 s build, agrees with
`ffmpeg` to **0.01 LU** — so the two ratchets state the same reasoning instead of one holding it and the other pointing
at it.

**Critique of the failure's cause.** Iteration 77 added the dependency correctly, added it to `ALLOWED_DEPS` correctly,
and missed the second guard. That is not carelessness about the dependency; it is the mechanical consequence of having
**two** guards and a suite that could not run. The tests' own messages name each other and say *"in the same commit"*,
which is why the fix took one edit.

**Verdict: GREAT.** The red is gone, the two guards agree, and the reasoning did not get duplicated into a place that can
drift.

## Pass 2 — Zoom: the measurements

**Critique.** Five iterations of owed numbers, all taken now on an idle M4:

| | |
|---|---|
| `cargo test` | **198 passed** |
| `--features serde` | **207** |
| `--features mixer` | **208** |
| `--features doc` | **6** |
| `--features semantic --lib` | **20** |
| clippy `-D warnings` | **0 warnings** |

**Perf, case 19** — 0.084 / 0.472 / 1.576 µs per frame at 8 / 64 / 256 voices. The per-voice cost **falls** with count
(0.0105 → 0.0074 → 0.0062), which is what a fixed per-frame cost amortised over more voices must do; a figure that rose
would mean per-voice work had appeared.

**The visual gate, read rather than tabulated.**
- `bang-spectrum.png`: a broadband transient filling the first ~35 ms, then discrete harmonics decaying out of the
  2–7 kHz band by ~0.21 s. **No vertical line at t=0** (that would be a click), **no flat shelf at 0 dBFS** (clipping),
  **no hole** (a dropped voice). The harmonic comb is visibly denser than the footstep's, which is the difference the
  two cues are supposed to have.
- `bang-attack.png` and `bang-tail.png` at 50 ms and 100 ms windows: the transition from noise-like onset to tonal decay
  is smooth across the whole frame, with no step discontinuity at the 0.035 s boundary.
- `footstep-grass-spectrum.png`: narrower partials, faster decay, same absence of artifacts.
- Master mix: **−21.5 LUFS, −7.5 dBFS** — identical to iterations 67, 71, 74 and 75.

**Before/after is byte-identical.** `cue-bang-0.wav` and `mix.wav` hash the same in `evidence/before/` (iteration 75's
render) and `evidence/after/`. That is a stronger statement than "unchanged", because it is a proof rather than a
measurement: the audio path could not have been touched.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: a different cue set, and does the disk fix hold?

**Critique.** Re-rendered with `--voices 64` into `evidence/fresh/`: **32 files**, and `voices-64.wav` measures
**−10.0 LUFS, +2.9 dBFS** with **290 samples past full scale**. That is not a defect — `render` says so itself:
*"the budget is a sample cost, not headroom: `SirenMix::worst_peak_db` is how a game asks"* — and the waveform shows the
solid block that 64 summed voices produce. A renderer that hid this would be the defect; this one reports it in the
line it writes.

**The disk fix, which is this iteration's other deliverable.** Six iterations lost steps to `errno=28`, and the two
before this one spent their time on `cargo clean -p` of **dependency** artifacts — the wrong lever, which made each
subsequent build need a fresh copy. The right lever is the crate's own output:

    cargo clean -p bevy_siren --target-dir target   →   6.3 GB, no dependency touched

That took the volume from **122 MB to 6.2 GB** and every step above ran. It is written into `plan.md` and into this
iteration's log entry so the next agent reaches for it first.

**Can a developer follow it?** The failing test's message names the other file and the remedy; the fix is one string;
`eval.md` shows the red before and the green after. Nothing needs the code explained.

**Verdict: GREAT.**

## Note on the five missed iterations

The loop's rule is *"re-run the whole baseline; a case that went red is this iteration's failure."* The disk made that
rule unenforceable, and a red shipped. The lesson is not "be more careful" — it is that **a baseline you cannot run is a
blocker of the first order**, ranking above every feature, and it belongs in the log's top item rather than being
absorbed iteration after iteration.
