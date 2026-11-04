# Iteration 110 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the
earliest pass it could affect.

## Pass 1 — Function

*Against `#38`'s three criteria and against `transformers`' own numbers; no cue-table case moves (`cues.yaml` 33 of 33 green,
unchanged — this iteration touches nothing on the mix, music or play path, and the 21 rendered WAVs are byte-identical to
iteration 109's).*

**Critique, first reading — five things that were wrong:**

1. **A claim written before it was measured.** I had put *"the tower is nearly all of it"* in `indexer.rs`'s throughput doc, then
   timed the stages: **35.1 ms tower, 26.0 ms this crate's own mel**, so the mel is 42 % of a clip. The sentence was a guess in the
   voice of a measurement. Rewritten with both numbers and what they are.
2. **The export script called `onnx` optional.** It is not: running the script in the other environment on this Mac
   (`~/skintokens/.venv`, transformers 5.14.1, no `onnx`) failed in `torch.onnx.export` with *"Module onnx is not installed!"* —
   so the `try: import onnx … except ImportError` I had written could never reach its `except`. Now a hard import, the docstring says
   what the script needs, **and that it has run in exactly one environment** (Python 3.9, torch 2.8.0, transformers 4.57.6,
   onnx 1.19.1). It was **not** verified on transformers 5.x — the venv cannot export.
3. **"A few minutes on a laptop CPU"** — a time I had not measured. The export takes **6 s** here. Removed.
4. **The README's five-minute figure sat beside *"a re-index pays only for what changed"*,** which reads as if the example did
   that. `incremental::update` does; `examples/semantic` rebuilds the whole folder whenever it changes (the library has no
   model-backed form of `update`). README, guide and CHANGELOG now say so.
5. **`#38`'s second criterion** — *"no GPU prints what is missing and exits non-zero"* — cannot be taken literally: this build has
   no CUDA path (`Device::available()` is `Cpu`, unconditionally), so a GPU error would be every run. Read as *"a run that cannot
   work says why and exits non-zero"*: a missing model is the failure, the first line says the run is CPU fp32. **Recorded as an
   assumed decision** (`plan.md`, the guide's "Try it", the issue comment).

All five fixed; the tests and the real runs re-done after (`parity.txt`: **4.89 × 10⁻⁴** worst, same ranking order, with the
*re-exported* graph).

**Criteria, as met:**

| `#38` criterion | evidence |
|---|---|
| `cargo run --release --features semantic --example semantic -- <dir> "<description>"` indexes and plays | run verbatim through `cargo run`: 14 indexed, `Played audition.89f4c994c2850cba on the Music bus at +0.0 dB, variant Sosumi.wav`, exit 0 (`play-real.txt`) |
| with what it needs missing, prints what and exits non-zero | `refusal-no-model.txt`: exit **2**, three files named, **nothing written, nothing played** |
| listed in `docs/media/README.md` with the command that regenerates its picture | yes; the commands were run as written to produce `semantic.txt` and `semantic.png` |

**Verdict, first reading: NOT YET** (items 1–4 were claims that outran their evidence). **After the fixes: GREAT.**

## Pass 2 — Zoom

*Files read: `evidence/spectrograms/mix.png` (the master, from the render gate), `docs/media/semantic.png`, and two 2× crops of it
(`semantic-zoom-top.png`, `semantic-zoom-bottom.png`).*

**Critique.** The master spectrogram is the harness's: three sustained tonal lines under 1 kHz and one broadband onset in the first
0.04 s, no clipping streak, no gap, no spike — and it **is byte-identical** to iteration 109's WAV (`cmp`, 21 of 21), so it can
only look the way it did. Loudness **−21.5 LUFS**, true peak **−7.5 dBFS**, unchanged. The still, at 2×: the amber `Played` and
`done` lines are distinct from the grey reasons; no clipped glyph, no tofu for the em dash or the ellipsis; the longest line
(the `semantic:` header, 150 characters) ends 30 px inside the right margin. One honest oddity: the picture's first run reads
**12.7 clips/s** and the guide says **16.6**. The picture is 14 clips, so the first call's session warm-up weighs more; the 280-clip
figure is the stable one and the guide says which it is. The media README says timings differ run to run. Not separately measured.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*A different library and different phrases, then every option and error path — `evidence/after/edge-cases.txt`.*

**Critique — what a stranger would hit, found by running it:**

1. **`--k 101` was told *"A k of 0 asks for nothing"*.** `ToolProblem::BadK` ended in one sentence for both cases; an agent over
   `siren-tools` reads exactly that message. A library defect, found from the example's surface. Fixed in `tools.rs`;
   `a_bad_k_is_explained_by_its_own_case` **fails on the old message** at `tests/semantic_tools.rs:530` (run, not assumed) and passes
   on the fix.
2. **An empty description was refused after the 501 MB text graph had loaded** — the cost the pre-flight exists to avoid. The
   description is now validated with the other arguments, before the folder is scanned; the run prints no `graph loaded` line.
3. **The transcript needs the code to explain it.** *"semantic carried it … no value for acoustic, tempo, key"* and *"unknown, source
   not recorded"* mean something only to a reader of `explain()` and `Provenance`. The guide now says what both lines are, and that
   an `anchor` brings the acoustic terms in.
4. `--pick 2 --bus Effects` plays the second result on the **Effects** bus; `--bus Reverb` is answered by the crate's own play path —
   `Refused audition.89f4c994c2850cba: bus `Reverb` is not in the mix table`, exit 1 — one of the two answers a play must give;
   `--pick 6 --k 5`, a missing directory, an empty one: exit 2, 2, 1, each with a sentence.

**Not covered, said plainly:** no test runs the example or the script (CI has no model), so a regression in them is caught by the next
person who runs them, not by CI. The library halves they use are tested; the example's own guards — pre-flight, refusing an empty
index — were exercised by hand and recorded. A person has not **heard** the played result; the evidence is the crate's `Played` and the
voice ending by itself, with no *"no audio device"* warning.

**Verdict: GREAT** (after fixes 1–3, each re-run).
