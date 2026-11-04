# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **The golden set's one library told one story about the similarity floor, and a second library told another.** Iteration 111 recommended `0.35` from
  2 000 recordings; iteration 112 withdrew it on the strength of the repository's own demo (nothing scored above 0.256); **iteration 113 measured the second
  library, and the doubt was right by more than a doubt**: on 303 game sounds the right answers' median similarity is **0.191** against 0.468, the weakest answerable
  description's best result is 0.261 against 0.492, *a helicopter* finds an `engine` at **0.490** — above the best result of 27 of 29 answerable descriptions —
  and **`0.35` returns nothing at all for 13 of 29**. The largest floor that costs neither library an answer in either wording is **0.20**, and it silences no description
  that should find nothing. The default is unchanged at `0.0`; `needs-decision/default-floor.md` now recommends keeping it; `Floor::min_score`, `Weights::min_similarity`,
  `docs/guide/semantic.md` and the README no longer offer `0.35` as the answer. **Three of the eight predictions written down before the run were wrong**: retrieval is
  31 % at hit@1 where 60 – 85 % was predicted, and centring each query on its own median *lowered* the AUC by 0.13 where a rise was predicted. **How sure**: a bootstrap over the
  descriptions puts the two libraries' AUCs apart only in the maintainer's wording (0.59 – 0.90 against 0.90 – 1.00); what separates them in both wordings is a count — **13 of 29
  descriptions have a best result under 0.35 on Kenney (Wilson 95 %: 28 – 62 %), 0 of 50 on ESC-50 (0 – 7 %)**.
- **Two of the five formats the scan calls audio could not be decoded, and nothing had ever tried.** `library::AUDIO_EXTENSIONS` —
  *"the formats the crate can decode"* — lists `wav ogg oga flac mp3`; `rodio` was given `wav` and `vorbis`; `indexer`'s own error says
  *"a library of `.mp3` files is indexed"*. Every `.flac` and `.mp3` was scanned as audio and refused (*"The format of the data has
  not been recognized"*) because **no test had opened a file that was not a WAV**. `rodio` now has `flac` and `mp3` — two pure-Rust crates:
  `--features semantic` goes from 487 to **497** lines and 181 to **183** crates; the default build is unchanged at 413 and 149 — and
  `every_extension_the_scan_calls_audio_has_a_decoder` fails if the list and the decoder drift again. **Checked against decoders that are
  not this crate's, on real files, through the whole pipeline**: 2 000 ESC-50 clips as FLAC give *identical* similarities to the WAVs (all
  232 000, largest difference 0.0); Kenney's 62 MP3 effects match `ffmpeg`'s decode on all 259 description–file pairs, every length exact;
  209 Ogg effects match `libvorbis` to 1.8 × 10⁻⁷ relative rms. The claim in `decode.rs`, `tests/leaf.rs` and `tests/library.rs` that the
  decoder *"costs no new crate"* was never true — `rodio` is in the default graph and its decoders are not, 16 of the 34 crates `semantic`
  adds — and is corrected with the decomposition.
- **`library::read_header` read the wrong bytes for Ogg and FLAC.** A 48 kHz mono FLAC reported **768 000 Hz** (the 20-bit rate field was
  shifted by the wrong amounts) and an Ogg's rate was read three bytes late; only the WAV branch had a test. Fixed, and tested against
  real `ffmpeg` files at two rates and two channel counts rather than a header built from the same reading of the spec as the parser.
- **An Ogg Vorbis file decoded up to 20 ms too long**: `symphonia` keeps the padding of the last block, which the spec says to discard.
  CLAP repeats a short clip to fill its input, so on the RPG pack's effects that tail alone moved a similarity by up to 0.085 and changed
  the top three of **all 12** descriptions tried (59 of 209 files were affected, by up to 975 frames). `decode` now cuts an Ogg that is one
  stream ending cleanly to the length its last page declares; against `libvorbis` the worst difference is 0.004. A chained file is left
  alone, and a test fails if that guard is removed (checked by weakening it). `mixer` plays through `rodio` directly and is unchanged. The
  same decoder clamps its Vorbis output to [−1, 1] — I first read the 22 of 51 files that differ as wrong samples, and was wrong: their
  decoded peaks reach 1.3, and clamping `libvorbis`'s output the same way makes all 209 agree to 1.8 × 10⁻⁷. A design choice, left alone.
- **`examples/semantic` cut a long compressed file off and said *done*.** It waited for a length estimated as 16-bit PCM from the header: a
  10 s FLAC of a steady tone was waited 1.7 s for, a 15 s MP3 5.8 s. It now waits for the decoded length (9.9 s and 14.8 s).
- **The recommendation in `needs-decision/default-floor.md` was contradicted by this repository's own second library, and was not revised.**
  Iteration 111 recommended a floor of `0.35`; iteration 110's demo — fourteen macOS UI sounds — scored **nothing above 0.256** for any of five
  descriptions ("a short electronic beep" → `Sosumi` 0.206, "a glass being tapped" → `Glass` 0.016), so every positive floor would have returned
  an empty list. It now recommends keeping `0.0` until a second library of game audio is measured; the guide and two doc comments say so. The
  default is unchanged.
- **The golden set's baseline is the WAV encoding, and the encoding moves the answer** — stated where the numbers are. The same clips as FLAC
  are identical; as MP3 at 128 / 64 kbit/s, hit@1 goes 90 → **94** / **98** %, zero-shot 87.8 → 87.5 / 87.2 %, and the stranger's hit@10 92 → 92 / 88 %.
  The prediction written down before the run said hit@1 would move by at most two points; it moved four and eight, and the typical similarity by
  0.005 and 0.033. A change in hit@1 under about eight points is within what re-encoding the library does.
- **The documented basis for the similarity floor was false on real recordings.** `Floor::default()` is `0.0`, justified as *"a
  measurement, not timidity"*: six synthetic clips whose right and wrong score bands overlapped, so *"no single threshold separates
  them"* and *"a floor of 0.5 would reject every correct match"*. The first run of the golden set — **2 000 real recordings, 50
  descriptions** — measured the opposite: right answers' median **0.468** against the wrong answers' **0.070**; the bands overlap
  only in their tails (right p05 0.273, wrong p95 0.277); the weakest *top result* of any description scores **0.492** (**0.405** when
  the same sounds are described in a stranger's words, `queries-paraphrase.tsv`); a floor on the top result separates a description
  the library can answer from one it cannot with an **AUC of 0.963** (0.885 paraphrased); and a floor of **0.35** keeps every
  description's top result in both wordings, returns 55–66 results a query at ~50 % precision (not 1 318–1 440 at 3 %) and silences 4
  of 16 descriptions of sounds the library lacks — while **0.45, the knee in the maintainer's wording, already drops two paraphrased
  answers**. The six synthetic tones' *best* match (0.429) is below the real median — synthetic audio understates a real match, so a
  floor calibrated on it was calibrated on nothing. The claim is corrected where it was made (`rank.rs`, `weights.rs` ×3,
  `ensemble.rs`, the guide's banner, floor and weights sections, `semantic-config.md`). **The default itself is unchanged** and is now
  `docs/loop/needs-decision/default-floor.md`, with the table and a recommendation (0.35).
  **Four tests that pinned the old story are deleted**: `the_measured_bands_overlap_which_is_why_the_default_floor_keeps_everything`
  (a compile-time assert on two constants, and a pin of the default), `an_intuitive_floor_would_have_rejected_every_correct_match` and
  `a_floor_at_the_measured_boundary_keeps_the_best_and_drops_the_worst_miss` (arithmetic on literals, the first concluding something
  now false), and `the_default_floor_keeps_everything_because_no_labelled_set_exists_yet` (a labelled set exists).
  `a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model` is now `a_floor_drops_a_right_answer_that_scores_under_it`,
  an arithmetic test with no narrative. Also: `semantic/mod.rs`'s stage table still said *"index and query: not yet"*; it is now a true
  module map.
- **Two claims about the semantic layer that its first real run falsified.** The README said *"a 5 000-clip library costs
  seconds, not minutes"*, and the README and the guide said `cargo tree` reads *"the same 413 lines"* with and without
  `semantic`. Run end to end on the real CLAP graphs for the first time, the audio tower indexed **280 clips at 16.6 clips/s**
  on an M4 CPU — **60 ms a clip, 35 ms of it the tower and 26 ms this crate's own mel spectrogram** — so 5 000 clips take
  about **five minutes**; the 750 clips/s figure is decoding alone. And `semantic` takes the graph from **413 to 487 lines**
  (149 crates to 181), which was true only of the build that does not enable it. Both are corrected where they were said,
  with the measurement, and `indexer`'s throughput doc gained the CPU-inference row it lacked. **And the five minutes was for
  one-second clips**: 2 000 five-second 44.1 kHz recordings cost **105 ms** a clip (9.5 clips/s, under load), about nine minutes for
  5 000 — a clip's cost is not constant, and the extra 45 ms is not yet decomposed.
- **A `k` above the cap was told *"a k of 0 asks for nothing"*.** `ToolProblem::BadK`'s message ended in one sentence for both
  of its cases, so a caller that had asked for **101** — an agent over `siren-tools`, where the message is all it reads — got
  advice about zero. Found by running `examples/semantic --k 101`. The message now explains the case it is (*"k is a bound, not a
  page: a call returns at most 100"*), and `a_bad_k_is_explained_by_its_own_case` fails on the old text at *"at most"* — run
  against it, not assumed.

- **`explain_match` explained an effect with the wrong weights.** It was hard-wired to the music weighting, so a pair
  `find_compatible_sfx` had ranked — under the *effects* weighting — came back with a different score and different weights:
  numbers individually right that did not add up to what the search returned. The first real session through
  `siren-tools` showed it plainly, `acoustic` at **1.2** in the search and **0.5** in the explanation of the same pair.
  `Toolset::explain_match_with(…, &Weights)` takes the weighting that ranked the pair (`explain_match` keeps its signature
  and the music default), and the tool gains an optional `weighting: "music" | "effects"`. Proved by making the handler
  ignore it: `explaining_an_effect_uses_the_weighting_that_ranked_it` fails at *"under the weighting that ranked it, the
  explanation is the result"*.
- **Four statements that were false, in documents a reader trusts.** `indexer::write`'s doc and `incremental`'s said the
  index is `siren-index.txt`; `INDEX_FILE` is `siren-index`, with no extension, one name for both formats — a person looking
  for the file by its documented name would not find it. ADR 0007 said the tool server *"cannot be built in this crate yet"*
  and pointed at a decision file, but ADR 0006 had already decided the layout and said the dependency is argued in a
  `leaf.rs` row, so nothing was waiting on anyone (`needs-decision/mcp-tool-surface.md` now says so). `AGENTS.md` listed the
  closed dependency set as four names while `ALLOWED_DEPS` held seven. And `tests/library.rs` and `tests/leaf.rs` each
  carried a count in prose (*"exactly four"*, *"only the four"*) beside the list it described — the drift iterations 106–108
  were about.
- **The table test covered six of `CueProblem`'s seven variants and stayed green.** `InertAvoidRepeat` was tested on its own
  (`tests/play.rs`, `tests/model.rs`) but no fixture cue in `problems_names_each_mistake` produced it, so the test whose job
  is *every problem prints as a sentence naming its cue* never reached it. The compiler had already demanded an arm for it
  when it was added (`E0004`, iteration 105) and an arm is all that was written — **an exhaustive `match` forces
  acknowledgement, not coverage**. `tests/table.rs` now lists every variant **once**, in a `kinds!` macro that generates the
  exhaustive `match` and the count together, and fails with `no fixture cue produces ["InertAvoidRepeat"]` until a fixture
  cue produces each kind — proved by replaying iteration 105's state on the real variant. A test-only change; `src/` is
  untouched. Same repair as `mix::SETTINGS_CHECKS` and `music::MUSIC_CHECKS`, for the enum-returning sibling.
- **`MusicScore::problems()` claimed a complete list and nothing enforced it.** Its doc said *"Nine checks, and this list
  is the whole of them"*, and a test asserted each of the nine fires — but the nine were correct **by hand**: adding a tenth
  check to `problems` left the suite green while the doc went on claiming completeness. `music::MUSIC_CHECKS` is now the
  one list the doc names and the test walks, proved by adding a tenth check **and its name** and watching it fail at
  `9 != 10`. Same repair as `mix::SETTINGS_CHECKS`, applied to the sibling type.
- **`SirenSettings::problems()` advertised a guarantee nothing provided.** Its doc said *"Twelve checks, and
  `tests/mix.rs::problems_names_every_mistake` walks all twelve so this list and the code cannot drift apart"*. Measured:
  the code performed **thirteen**, the test asserted **eleven**, and the prose said twelve — and the check missing from
  every list was the `by_db == 0.0` one, written for a **measured** bug. `mix::SETTINGS_CHECKS` is now the **one** list:
  the doc names it instead of restating it, and the test asserts that every name has a needle or a named dedicated
  fixture — proved by appending a fourteenth name and watching it fail. Adding the `by_db(0.0)` duck to the fixture also
  means that check is finally exercised by the test the doc pointed at.
- **`Repeat::AvoidRepeat { avoid: 0 }` refuses nothing** and did so silently. `avoid()` returns the field verbatim and the
  picker refuses `min(avoid, len)` recent sounds, so a window of zero is **byte-identical to `Shuffle`** — proved by two
  pickers on one seed producing the same 24-pick sequence — while the variant's own doc promised it *"kills the
  machine-gun footstep"*. A generated table computing the window from a variant count (`avoid: variants - 1`) reaches it
  mechanically, and every play succeeds while a listener hears the repeat. `CueProblem::InertAvoidRepeat` now reports it
  with both remedies rather than coercing the number, because a legal table's behaviour is the author's to change.
  `Repeat`'s doc also now says `avoid` counts **sounds**, not slot indices.
- **Three defects in the tool surface.** (1) **The similarity floor was ignored**: `tools.rs` compared against
  `Floor::default().min_score`, which is `0.0`, so `Tally::below_floor` was documented, rendered by `describe()`, and
  **permanently zero** — and a caller had no way to set a stricter filter. `Weights::min_similarity` is now a field
  (**design default `0.0`**, `Floor::min_score`'s existing default, and `0.0` because the measured semantic bands overlap:
  a floor of 0.5 would reject every correct match). (2) **The category filter was case-broken**: `source_of` lowercased the
  haystack but the needle was compared raw, so `Some("Footsteps")` against a pack recorded as `Footsteps` matched
  **nothing** and the tally blamed the category. (3) **A bad compatibility multiplier named the wrong field**:
  `clash_penalty: 1.5` produced *"`drop_fraction` is 1.5"*, sending a person to edit a field they never touched. The two new
  `WeightProblem` variants that fix this break an exhaustive `match`, so the change is also filed in
  `needs-decision/weight-problem-names-its-field.md`.
- **`#20`'s stamp override could not be used.** `Index::overridden` was written by `load_allowing` and its own doc claimed
  every search would see it, but **nothing read it** and both search entry points refused on any stamp mismatch — so an
  index loaded through the override **could never be queried**, measured as `Some(StampMismatch)`. A user who followed
  the refusal's own advice (*"re-index"*) got a second refusal contradicting the first. `Index::accepts` is now the one
  rule both searches ask: the stamps match, **or** the index carries an override **granted for exactly that model** — so
  consenting to one mismatch is not consenting to a third model, which would be a standing permission to rank noise.

### Added

- **A second golden library — 303 game sounds from Kenney's CC0 packs — and `goldenbench --set`.** `golden/kenney/` holds a manifest pinning seven archives and 303 files by
  SHA-256, the labels (the author's own file names, grouped into 29 classes by a rule fixed before measuring), and **two files of 47 queries** — 29 with right answers, 18
  that should find nothing — written and committed *before the model ran*, with eight predictions; `tools/fetch_kenney.py <dir>` fetches the seven archives from
  OpenGameArt's static URLs, checks every archive and every file against this repository's hashes, and refuses a stray, damaged or wrong-bytes file by name (exercised against
  six failure modes). WAV, MP3 and Ogg Vorbis in one library, 221 of its 303 files under a second. **`goldenbench --set <name>`** generalises the benchmark with one baseline per set
  — **ESC-50's identity string is byte-identical and its baseline still compares: no regression, all seven metrics +0.0000** — and prints the separation (`eval::separation`,
  `eval::centred`, additive public items), a floor table on each query's own median, a precision@10 ceiling from the class sizes, and `--per-query`. A Python recomputation
  agrees on every printed number on both libraries and both wordings (it caught an `f32` AUC printing `0.962` for an exact `0.9625`; now an `f64`); against `transformers` the
  48 kHz files are identical and the 44.1 kHz ones differ by a resampler's amount (mean 0.004). The findings are in `docs/guide/golden-set.md` ("The second library") and
  `needs-decision/default-floor.md`. `tests/semantic_golden.rs` +12 tests.
- **`#35`'s golden set — recall, precision and the floor, measured on 2 000 recordings people labelled.** `golden/esc50/` holds a
  manifest (SHA-256 of every file at a pinned commit), **two files of 66 text queries** — 50 with right answers and 16 that should find
  nothing, each with a one-line rationale; one in the maintainer's words and one paraphrased to avoid the class names — and a recorded
  baseline; `tools/fetch_esc50.py <dir>` fetches the 2 000 clips **file by file, 16 at a time, each
  checked against the manifest as it arrives** (79 s; the one-archive route measured two hours) and writes `labels.csv` and the
  `licences.csv` sidecar the crate already reads — failing unless every clip has a licence row. `semantic::golden::GoldenSet::parse`
  refuses a mistake **by line** (no rationale, a repeated description, a class no file is in, a header that names no columns);
  `semantic::eval` gains set-valued relevance (`ScoredCase`, `hit`/`precision`/`reciprocal_rank`/`average_precision`, `score_cases`,
  `floor_curve`, `score_bands`) — all additive. `examples/goldenbench` indexes the set with the real audio graph and prints the paper's
  zero-shot accuracy, retrieval hit@1/3/10, precision@10, MRR, mAP@10, the score bands, the floor curve, which sound each no-answer
  query found instead, that `search_music` returns the same ten as the raw ranking, and a baseline compare. **Results:** zero-shot
  **87.8 %** (the paper: 88.0 – 91.0 % on an overlap-excluded subset) — and an independent `transformers` run on the same 44.1 kHz audio
  gets **87.8 %** too; in the maintainer's words hit@1 **90 %**, hit@3 **100 %**, hit@10 100 %, precision@10 88.8 %, MRR 0.950, mAP@10
  0.857 — and **in a stranger's words 72 % / 84 % / 92 %, 73.6 %, 0.791, 0.676**; the crate's 232 000
  similarities differ from the reference's by 8 × 10⁻⁴ on average and 0.089 at worst (resampler against resampler; the one that
  matches the model's training pipeline is unmeasured). On the real sidecar the crate's provenance reads **1 770 licences that permit
  commercial use, 170 that do not, 60 unknown** — unknown never shown as permissive. **Not met, and not a machine's:** `#35`'s third
  criterion, anchor pairs judged by a person. ADR 0009, `docs/guide/golden-set.md` (including what it cannot say: ESC-50 overlaps the
  model's training data, one library, no music), `tests/semantic_golden.rs` (19: the metric arithmetic, the loader's refusals, and the
  checked-in set's own invariants).
- **`examples/semantic` and `tools/export_clap_onnx.py` — the semantic layer, run end to end on real files** (`#38`).
  `cargo run --release --features semantic --example semantic -- <folder> "<description>"` builds (or reuses) the index,
  embeds the description, prints the top results **with the reason each one ranked**, and plays one through `SirenPlugin`'s
  own `Play` path — `Played audition.89f4c994c2850cba on the Music bus at +0.0 dB, variant `Sosumi.wav``. With no model it
  says exactly which files are missing and exits **2** before it reads the library, writes an index or opens the output
  device; there is no GPU path, and the first line says the run is CPU fp32. The export script makes the two graphs and the
  tokenizer from the weights in a Hugging Face cache and **checks itself**: the text tower against `get_text_features`
  (largest difference **0.0**), the audio tower against `audio_model(is_longer=false)` (cosine **1.000000000**), and
  `onnx.checker`. Against `transformers` on **5 phrases × 14 real files** the example's similarities differ by at most
  **4.9 × 10⁻⁴** — the three printed decimals — and rank in the identical order. Two things only a real run could show:
  **`transformers`' `get_audio_features` is not the path the crate runs** — its extractor marks one clip of every fused batch
  `is_longer = True`, even a lone one-second clip, and that moves the embedding by cosine **0.72–0.83** — so an export traced
  with the extractor's tensor baked in a branch the crate's `false` then ran on an empty batch (*"Input shape
  {0,96,64,3,21}, requested shape {0,96,64,-1}"*), and the **first real run indexed 0 of 14 files and wrote an empty index**.
  The script now traces `is_longer = false`, keeps it a real input (the exporter drops an unused one, and the crate sends it
  by name), and the example **refuses to write an index with nothing in it**. It treats `siren-index` as a cache, so a
  changed folder is **rebuilt whole** — `incremental::update` takes the embedder as a closure and has no model-backed form
  yet. The first real picture is `docs/media/semantic.png`, regenerated by the commands in `docs/media/README.md`.
- **`ModelStamp::clap(version)`, and `--model-version` on `siren-tools` and the example.** An index records which export made
  its vectors and a mismatch is refused, but the only stamp a caller could build was `measured_clap()` — the author's torch
  and transformers — so the first real export on another machine (`transformers-4.57.6/torch-2.8.0/opset-17`) was a
  mismatch with no way past it. `ModelStamp::clap(v)` names any export (`measured_clap()` is `clap(MEASURED_CLAP_VERSION)`),
  and `--model-version <v>` (or `SIREN_MODEL_VERSION`) is the flag; an index from another version is still refused, naming
  both. Additive. Tests: `a_stamp_names_the_export_that_made_the_vectors`,
  `an_index_from_another_export_is_served_once_its_version_is_given` (a real `siren-tools` process, refused without the flag
  and served with it).

- **`siren-tools` — the tool surface, for an agent** (`#28`). An **MCP server over stdio**, in this crate behind `semantic`
  as ADR 0006 decided: `cargo run --release --features semantic --bin siren-tools -- <library-dir> [--model … --vocab …]`.
  Six tools (ADR 0007), one error shape, and **a bad call is a tool result a model can read** (`isError: true`,
  `{"error": {"code", "message", "retryable"}}`) rather than a protocol error it never sees. The protocol lives in
  `semantic::server` so a test drives it with no process; **one table, `SPECS`, draws the listing, the schema and the
  validation**, so what a client reads and what its call meets cannot disagree — proved by loosening the schema by one and
  watching `the_schema_is_what_the_call_enforces` fail. Hand-written JSON-RPC with `serde_json` as the only new dependency:
  **6 crates in a probe against 58** for `rmcp` + `tokio` + `schemars`, for a host whose work is synchronous
  ([ADR 0008](docs/adr/0008-tool-host.md)). Measured by the new `examples/hostbench`: **34 ms and 20.5 MB to start** at
  10 000 assets, then 0.02 ms for `set_anchor`, 5.3 ms for a search (before the text encode) and 18 ms for
  `find_compatible_*`. `cargo build` without the feature builds no binary. 40 tests, including the binary as a process:
  stdout carries only messages, a missing index exits 1 with nothing on stdout, and stdin closing ends it with status 0.
- **A busy GPU is waited out, bounded and visible** — `#37`. `budget::Deferral { wait_s: 30.0, poll_s: 0.5 }` (both
  **design defaults**, and bounded because the source requires bounded latency), `budget::WaitState` (the value a panel
  renders as *"waiting for the GPU (2 of 60)"*), and `budget::defer`, which measures, waits and either returns the attempt
  that succeeded or a **named refusal** carrying both numbers. `Session::admit_deferring` wires it to a real session. The
  policy is **defer** rather than queue or shrink, argued against the other two: shrinking cannot load 623 MB into 60 MB,
  and an unbounded queue leaves the caller with no answer. `measure`, `sleep` and `report` are the caller's, so the
  function decides and performs no I/O — which is how a test proves a thirty-second bound without taking thirty seconds,
  and how `a_query_still_answers_while_the_build_waits` runs a real `top_k` between two polls.
- **The tool surface's error shape and idempotency table** — `#31`'s library half. `tools::Code`
  (`bad_argument`/`refused`/`unavailable`/`internal`) with `Code::retryable()`, and `ToolProblem::code()` mapping every
  failure onto it: **only a bad argument is retryable**, because a refusal is a fact about the state and a retry with the
  same arguments fails identically. `tools::Idempotency` and `tools::TOOLS` — six tools, each with its statement, and
  **`audition` the only unsafe one** because it starts a voice. A test enumerates the table, so a tool added without a row
  fails rather than going silent.
- **A dry/wet estimate** — `#23`. `semantic::reverb` measures ISO 3382's **C50**, the early-to-late energy ratio with a
  **50 ms** boundary (`early_window_s`, from the standard) over a **500 ms** late window (**design default**). A game asset
  is not an impulse response, so the **loudest frame is the direct-sound proxy** — stated as the approximation it is.
  Frames come from `analysis::spectra`, so it costs no second transform. A zero tail reports a finite
  `Reverb::CEILING_DB` (60 dB) rather than an infinity, which would compare false against every threshold; a clip too
  short to have a late window reports **no measure**, which is a different fact from being dry. Discrimination is
  recorded and printed every run: **12/12 synthetic pairs separated, dry ≥ 60 dB, wet ≤ 1.9 dB, bands disjoint** — and
  `DISCRIMINATION_MEASURED` says in its own text that **real recordings are not measured**, which is why the signal is
  **not in the default weights**.
- **The indexing throughput target** — `#36`'s measurable half. `indexer::INDEX_TARGET_CLIPS_PER_SEC = 700`, against a
  measured **748 clips/s**, enforced by a release-only test. The constant's doc states what a "clip" is (0.5 s, the
  length `indexbench` generates — the figure is per clip, so a 4 s clip measures ~83), that the **inference target and
  peak VRAM are not set** because no GPU host here has the graph, and that inference dominates on CPU by ~100× (ADR
  0002: 34.9 ms against 0.37 ms), so the GPU figure is the one that matters and is the one still owed.
- **`tools::search_music`** — `#29`'s library half, and `tools::DEFAULT_K = 3` (*describe, see three, pick one*). Each
  result carries its identity, score, breakdown, `explain()` sentence and provenance, so an agent chooses **without a
  second call**; identity is the content hash and never a path, which would be absolute to the machine that indexed it. An
  empty library is an empty list with a reason rather than an error, and `k` is bounded by `MAX_K`.
- **`tools::audition`** — `#30`'s library half. Given an indexed asset, a library and a bus, it returns an `Audition { cue,
  play }`: a cue named `audition.<hash>` (so the same asset always auditions under the same name) with **one** sound slot,
  and **exactly one** `Play` for the caller to send. It never plays anything itself, so `play_requests` remains the only
  place a voice starts. `AuditionProblem` distinguishes an asset that is not indexed from one the library no longer holds,
  because the two need different fixes.
- **`tools::MAX_K` and `tools::check_k`** — ADR 0007's bound: every list-returning tool returns at most `k`, `k` is
  capped at **100** (**design default**), and a bad `k` is refused *before* any work happens. `ToolProblem::BadK` names
  the valid range. A `k` with no ceiling is an unbounded list with a friendlier name.
- **The human listening check** — `#46`. `docs/guide/listening.md` is the procedure: the commands, the **order** of the
  five questions, what counts as a failure and what the follow-up is, and the shape the outcome is recorded in.
  `examples/audition` writes an anchor's neighbours as `01-first.wav … 0k-last.wav` — **rank order, not alphabetical**,
  because the question the check answers is whether the ranking put the right thing first — with a `README.txt` naming
  each file's hash, the stamp and the weights. It copies rather than re-encodes, so a listener hears the asset as it is.
- **The device path is reported, and it is the CPU path** — `#19`'s first criterion. `Model::device()` returns a
  `Device`, and `Device::describe()` says *"CPU, fp32 — no CUDA device in this build; an index built on a GPU may rank
  differently here"*. `Device::available()` is `Cpu` unconditionally: `ort` 2.0-rc.15 has **no CUDA feature**, so there
  is no second path to choose, and reporting one would be the same silent lie `#19` forbids in the other direction.
- **Batched embedding with a visible shrink** — `#18`'s host half. `session::embed_batched` embeds a list in batches up
  to a configured ceiling, **halves** on a refusal and continues, records the sizes used (`Batched::batches`) and the
  number of shrinks, and **recovers toward the ceiling** afterwards so one early refusal does not cap the run. A refusal
  at a single item is a **failure**, not a shrink: retrying smaller is impossible, so it is reported rather than
  returning fewer vectors than were asked for. A short answer from the embedder is refused for the same reason — it would
  pair vectors with the wrong assets from that point on.
- **Long tracks are chunked by the crate's own policy** — `#13`. `semantic::chunk::Chunking` states a **10 s window**
  (the model's own) and a **5 s hop** (half the window, so every sample is inside two chunks and a boundary-crossing
  event is fully present in one), both on a serde type. `chunking.chunks(samples)` gives every window's start, length and
  offset, and the final chunk is clamped to reach the last sample so a tail is never dropped. `chunk::pool` combines the
  chunk vectors **weighted by each chunk's RMS**, dropping anything at or below `POOL_SILENCE_DB` (−60 dB) entirely and
  normalising the result — because a plain mean pulls a mostly-quiet track toward the null vector, which is the issue's
  own pitfall. `indexer::build_from_model` uses it for every track longer than the window.
- **A regression harness** — `#45`. `cargo run --release --features semantic --example evalbench` prints **recall@3** and
  **anchor agreement**, their deltas against `docs/loop/runs/2026-11-03-98/eval-baseline.txt`, and **the failing queries
  by name**; `--bless` records a baseline. It exits non-zero on a regression, and it proves it *can* fail by running a
  deliberately degraded corpus and checking the metric drops. The corpus is deterministic and synthetic because `#35`'s
  golden set does not exist yet: `corpus()` is the one function a labelled set replaces.
- **A stamp override that reports itself.** `Index::load_allowing` and `Index::from_bytes_allowing` take an
  `allow_override`; `load` and `from_bytes` keep refusing, unchanged. An overridden index carries the mismatch on
  `Index::overridden`, so every search built from it can say *"loaded with a stamp override"* rather than the fact
  existing only in a log.
- **The workflow as calls**: `semantic::tools`. `Session::set_anchor` holds one anchor per session object — no global,
  so two clients cannot interleave — and is idempotent by construction. `Toolset::compatible_music` and
  `compatible_sfx` rank an anchor's neighbours with the two **separate** weightings (`Weights::effects()` moves the
  weight key and tempo held onto acoustic agreement, keeping the total so scores stay comparable), suppress
  near-identical variants **after** ordering so the better-scoring one survives, and report what they left out in a
  `Tally` that names each count. `Toolset::explain_match` returns the same `Breakdown` a search produces for that pair.
- **The ensemble's scale is signed.** Every signal is now an agreement in `[-1, 1]` — the cosine unchanged, acoustic and
  tempo as `1 − 2·difference / tolerance`, key as `2·multiplier − 1`. A `[0, 1]` map was the first attempt and was
  wrong: on it no contribution can be negative, so `Breakdown::strongest()` could never report what counted *against* a
  result and `explain()` was a score with decoration.
- **The ensemble, wired.** `semantic::ensemble::breakdown` puts every signal on that one signed scale and weights it
  into `model::Breakdown`. `search::search_ranked` runs the whole path: stamp check, ranking, floor, the hard rules, then a
  **re-sort by ensemble score**, which is what makes a project's weights binding. Each result carries its own breakdown
  and an `explain()` sentence.
- **`model::Contribution` compares absences as equal.** A derived `PartialEq` compared `raw: NaN` against `NaN` and
  returned false, so two identical searches were never equal whenever a signal was absent — `#40`'s determinism
  criterion was unsatisfiable against the shipped type. The hand-written impl treats `NaN` as equal only to `NaN`.

### Changed

- **`spectral` and `tempo` now share one spectral pass.** They walked identical 1024-point frames with the same hop and
  each took the same FFT, so the transform was computed twice per frame. `semantic::analysis::spectra` computes it once
  and both consume it: **9.44 ms → 5.11 ms per 4 s clip (−45.9 %)**, and indexer decode throughput recovered from
  **419.8 / 568.8 to 698.6–784.0 clips/s**. Outputs are byte-identical — 21 rendered wavs, and every existing spectral
  and tempo test passes with no change to an expectation. `tempo` also stopped allocating two 16 KB vectors per frame.
- **`spectral::analyse_spectra` and `tempo::estimate_spectra`** are the new shared entry points; `analyse` and `estimate`
  are their unshared forms and answer identically.

### Added

- **The tempo and key filter rules**, completing `#27`'s set. **Tempo is a hard exclusion** — two tempi 38 BPM apart
  cannot play together, and the issue's 90-under-128 example is refused — while **key is a penalty**, because a modal
  piece sits anywhere and key is the least reliable feature here.

  **The 4.0 BPM tolerance is derived rather than chosen**: the estimator carries **±2 BPM** of uncertainty (ADR 0003's
  floor), so twice that is the width below which a difference may be entirely measurement error. Excluding inside it
  would reject tracks whose tempi merely *look* different. Asserted on both sides: **3.9 and 4.0 admitted, 4.1 and 38.0
  refused**.

  **The three states iteration 93 made storable are read correctly.** A tempo record with `bpm: None` means *judged and
  not rhythmic*; treating that as a tempo would compare every non-rhythmic asset against every other as though they
  shared one. `Features::bpm()` is `tempo.and_then(|t| t.bpm)`, so all three stay distinct — and a **judged-atonal** key
  record penalises nothing, just as an absent one does not.

  `Weights` gains **`tempo_tolerance_bpm: 4.0`** and **`compatibility`** — the named serde table `#25` already built, so
  the field is a reuse rather than a second definition. A project that wants clashing keys sets `clash_penalty: 1.0` and
  the rule stops penalising, which is `#27`'s overridability criterion satisfied by data rather than by a flag. Both new
  fields are validated: a negative tolerance excludes everything, and a multiplier outside `0..=1` inverts or rewards a
  clash.

### Documented

- **A build now produces tempo and key, and stores them.** Both features existed and `build` called neither, so a filter
  could never read them. `Index` gains `tempo` and `key` as a fifth and sixth optional section, in both formats, produced
  from the buffer `embed_measured` already decodes.

  **Three states, all preserved** — which is the point rather than a detail:

  | asset | tempo record | key record |
  |---|---|---|
  | a 4 s click track | `bpm: Some(120.0)` | present, *no key* |
  | a 0.1 s clip | present, `bpm: None` | **absent** — shorter than a chroma window |
  | never measured | absent | absent |

  A `None` *inside* a record and *no record at all* are different facts: `#27` reads the first as **unjudged** and the
  second as **unknown**, and collapsing them would make every short asset look unmeasured. An empty section is not
  written, so every index from before this iteration is byte-identical and loads.

  **The cost is real and is stated as the headline**: decode fell from **~1 150** to **420–569 clips/s**, measured twice,
  because tempo runs an autocorrelation and key an **8 192-point** transform over every file. That is the case this
  iteration targeted, so it is a trade — 13 s for 5 000 clips is still far below the ~3 minutes the audio tower would
  take — but it is the **third consecutive iteration where a feature added a pass**.

### Documented

- **Key and mode detection, measured rather than assumed out of scope.** ADR 0003 excluded key from the Rust path on the
  assumption of a detector *"wrong 40 % of the time"* — a figure nobody had measured. `semantic::key` implements
  **Krumhansl-Schmuckler** with the Krumhansl-Kessler profiles, and on a corpus of **24 keys** (every tonic, both modes,
  I–IV–V–I with the roots in the bass) is **100 % exact**.

  That is **synthetic tonal material** — the easy case, and the module says so rather than claiming a general accuracy. An
  atonal asset reports `Key::absent()` from three directions (a single impact, white noise, silence), and
  `Compatibility::multiplier(None) == 1.0`, so an unknown key never penalises.

  `Compatibility` is a **named serde type** with cited defaults — a clash multiplies a candidate's score by **0.5**, a
  parallel mode by **0.85**, a fifth not at all — which is `#25`'s third criterion.

  **Five defects between the first draft and that number**, each found by printing a value: a 1024-point window cannot
  see a pitch (2.85 semitones per bin at C4); 4096 cannot separate B4 from C5 (2.5 bins against a four-bin main lobe);
  the chroma's band edges were **half a semitone out of phase**, splitting C4's own partial between B and C; log
  compression flattened the vector; and the test's own fixture held a tonic pedal under the V chord — a **minor second**
  — so the passage genuinely contained as much B as C and "no key" was the correct answer.

  `needs-decision/key-detection.md` records the measurement and what would overturn the recommendation.

### Documented

- **A tempo estimate, with ADR 0003's ±2 BPM floor as a test rather than a claim.** `semantic::tempo::estimate` returns
  `Tempo { bpm: Option<f32>, confidence, octave_ambiguous }`. The corpus is **every whole BPM from 80 to 159 — 80
  tracks**, four times `#24`'s minimum, and every one lands inside the floor.

  **`bpm` is an `Option`** because `#24`'s own pitfall is an estimator that invents a tempo for a single impact. A
  steady tone and a lone impact both report **no tempo** with a low confidence, and `R-03`'s filter already treats a
  `None` as *unjudged* rather than as a mismatch.

  **The range is applied to the lags, not the answer**, so the octave error cannot be created by the search; and
  `octave_ambiguous` **flags** a subdivision rather than "correcting" it, because `#24` asks for the ambiguity to be
  countable and silently halving is the same mistake in the other direction.

  **A false positive that survived one fix, worth recording**: a steady 440 Hz tone estimated **150.1 BPM at confidence
  0.29** against a click track's 0.86, so confidence alone cannot separate them. The onset-ratio gate's first constant
  was `0.02` — correct arithmetic on the **wrong unit**, since `onset_envelope` returns FFT energy ~10⁶ times a windowed
  time-domain energy. Measured ratios are **0.28 (tone)** and **4.1 × 10⁵ (click)**; the gate is **2.0**, with both
  numbers in the constant's doc.

### Documented

- **Hard filters from the acoustic features.** `semantic::filter` turns the loudness and noise-floor measurements into a
  **policy** with a stated reason and a stated cost, rather than a term buried in a scoring function. Loudness is a
  hard exclusion — a 12 LU gap is not a ranking weakness, the two cannot play together — and the noise floor is a
  **penalty only**, because a library holds clean and noisy takes of the same instrument.

  **An unknown feature never excludes**, which is the module's whole design. A 0.2 s footstep has no integrated
  loudness at all: a property of the *measurement*, not the sound. Excluding it for failing a comparison it was never
  part of would silently remove the commonest asset in a library, so every rule returns `Admitted`, `Unjudged` or
  `Excluded` and only one of the three removes anything.

  `Weights` gains **`noise_floor_tolerance_db: 12.0`** — design default, twice the loudness tolerance because this rule
  penalises rather than excludes — with its own `problems()` check. The boundary is `difference > tolerance`, so **6.00
  LU is admitted and 6.01 refused**, asserted on both sides.

  A cascade reports *why*: `0 of 6 admitted, 6 for loudness. Every candidate was removed…`, so an empty result is a
  sentence rather than a mystery. `unjudged_loudness` is counted separately, so a library of short effects reads as
  *"all 3 admitted"* with a note that two were never judged.

  Refs #27 — tempo and key rules stay unbuilt until #24 and #25 land.

### Documented

- **Spectral centroid, rolloff and noise floor**, from the same FFT the mel path already uses. `semantic::spectral`
  answers what a sound designer means by *"the same world"* — brightness and cleanliness — and it survives an embedding
  where loudness is partly normalised away. Measured: a 1 kHz sine reads a centroid within **5 %**, a low-passed copy is
  darker than white noise in both centroid and rolloff, and digital silence reads **`SILENT_DB`**.

  **The centroid is an `Option`, and that is the design.** A silent clip divides by zero, so silence is a *state*, not a
  defaulted `0.0` — `0 Hz` is a real reading, since a DC offset measures **9.375 Hz** through a Hann window. Both index
  formats write `-` for it, so the two can never be conflated on disk.

  `ROLLOFF_FRACTION` (**0.95**) is exported, because a rolloff without its percentage is not a number. The noise floor
  is the mean power of the **quietest tenth of frames** against the clip's own loudest — a percentile, so marked
  **design default**.

  **A cost, stated rather than absorbed:** decode fell from **2 636 to ~1 150 clips/s** at 500 clips, measured three
  times, because this is a second FFT sweep over the same frames. A build now produces three more statistics per asset,
  and 5 000 clips still costs **6 s** against the ~3 minutes the audio tower itself would take.

### Documented

- **`examples/indexbench` — indexing throughput, split into decode and inference.** `#36` has been owed for three
  iterations while three costs were added to `build`, and one number cannot say which stage is the slow one. Measured on
  an M4, on a generated corpus so the figure is reproducible:

  | clips | decode | total |
  |---|---|---|
  | 200 | **2 411 clips/s** | 1 856 clips/s |
  | 500 | **2 636 clips/s** | 1 812 clips/s |
  | 1 000 | **2 724 clips/s** | 1 846 clips/s |

  **5 000 clips: 3 s** at the measured rate, stable within 10 % across the three sizes. `decode` **includes the loudness
  measurement**, because `embed_measured` measures on the decode side — so the cost iterations 85–86 added is inside
  that number rather than missing from it.

  The second stage is a **synthetic** 262 144 multiply-adds, labelled as such, because the real bound needs the model:
  ADR 0002 measured the audio tower at **34.9 ms/clip** on a CPU against a decode of **0.37 ms/clip**, so **inference
  dominates on CPU** by ~100×. It is a separate example from `perfcheck` on purpose, per `#36`'s own note: a per-frame
  cost and a batch job must never be read as the same number.

  A `bug` found by running it: the bench's own corpus varied frequency by `seed % 24`, so seeds 0/24/48 wrote
  **byte-identical** files — the scan correctly returned 24 assets from 200 files, and the report printed `24 / 24`
  because its denominator was derived from its numerator.

### Documented

- **A binary index, a third of the size.** `Index::to_bytes` / `from_bytes`, and `build` writes it: **10.28 MB** for
  5 000 clips against the text form's **31.88 MB**. That 3× figure has been quoted in four iterations' logs and asserted
  by nothing; it is a test now (`five_thousand_vectors_weigh_what_adr_0004_says`), and it is checked on **realistic**
  embeddings rather than on constants.

  The layout puts a `BSI2` magic first, then a `presence byte` before an integrated loudness — never a sentinel, because
  writing `-70` there would recreate the defect ADR 0003 measured three implementations making. The checksum covers
  **every preceding byte**, so a truncation in any section is caught by one rule.

  `read` decides the format from the **first four bytes**, not the file name, so an index written as text by an older
  build still loads. The file is `siren-index` now, and the name carries no claim because the bytes do.

### Documented

- **A build reads a folder's sidecar, so licences reach the index.** `Sidecar::read` was tested since iteration 82 and
  **nothing called it**. `build` now reads the sidecar before embedding and joins it to the scan — the only place the
  two identities meet, since a sidecar names **files** and the index keys by **content hash**:

      Asset.paths (relative, several per content) × Sidecar.records (names)  →  Asset.hash → Index.provenance

  A `licences.csv` beside the audio is enough; the named file's search result reports its licence and source, and every
  other file carries `unknown` and answers `None` to `permits_commercial_use()` rather than inheriting a folder-level
  claim.

  **Three facts that used to be silent are now reported:**

  - A record naming a file the folder does not have is counted in `Built::unattributed` and named in the summary. The
    commonest cause is a typo, and ignoring it makes a licence *look* recorded while being attached to nothing.
  - A sidecar that **exists and cannot be read** is carried in `Built::sidecar` with the reader's own words, while the
    audio still indexes. That is a different fact from *no sidecar here*, which is normal and says nothing.
  - A content that appears at several paths matches if **any** of them is named, because it is one asset.

### Documented

- **A build now measures loudness, from the decode it already has.** `loudness::measure` existed and was tested since
  iteration 77, and **nothing called it** — the index stored only what a caller supplied. `build` now measures every
  file it embeds and stores the result, sharing the decode: `embed_measured` decodes, downmixes and resamples **once**,
  then hands the buffer to `measure` and to the embedder. A second decode would double the cost of the operation the
  feature exists to make cheap on a thousand-file library.

  A **failed measurement is not a failed file** — the vector is the index's reason to exist — but it is **counted**:
  `Built` gains `unmeasured`, and `describe()` prints it only when non-zero, so a run that measured nothing cannot look
  complete.

### Fixed

- **A false block count in the crate's own documentation.** `loudness.rs` claimed a 3 s clip has **7** gating blocks,
  and its test message repeated it. 7 is the **non-overlapping** count; a 75 %-overlapping block advances 100 ms, so
  3 s gives `1 + (144000 − 19200) / 4800 = **27**`. The existing assertion was `blocks >= 7`, **which passes on 27** and
  therefore never noticed — a lower bound cannot detect a wrong sentence, only a wrong magnitude. Corrected in three
  places, with the arithmetic in the assertion's message and the doc table moved to 1 s, where 7 is the right number.

### Documented

- **Per-asset loudness, stored and round-tripped.** `Index.loudness` is a second optional section, the same shape
  `provenance` uses, and the last of #21's four criteria. Integrated loudness is stored as **`-` when it is absent**,
  and that is the whole design: ADR 0003 measured that for a 0.2 s clip `ffmpeg` reports **−70.0 LUFS** — the standard's
  floor presented as a level, and *also a real level* — while `ebur128` reports **`-inf`**. A format that writes a
  number there destroys a distinction the in-memory type was built to keep.

  Everything else in a record **is** a real measurement even for a clip too short to gate, so the true peak, the sample
  peak and both block counts are always written — and `too_short()` and `silent()` still disagree after a write and a
  read. An index with no loudness writes no section at all, so every previously written file is byte-identical and
  still loads.

  A record carrying a level that cannot be written back is **refused by name** at parse rather than repaired:
  substituting `0.0` would look like a measured silence. Refs #21.

### Documented

- **Per-asset provenance: where a sound came from and what its terms are.** `semantic::provenance` carries `licence`,
  `source` and `author` as `Option`s and reads the CSV and JSON sidecars a purchased pack ships. It is `p0` because it
  is cheap now and impossible later: an index built without provenance can only be recovered by rescanning files that
  may no longer exist.

  **`permits_commercial_use() -> Option<bool>` — three answers, not two.** `None` means *nothing recorded, or an
  identifier this crate does not know*, and it is left for a person. A `bool` defaulting to `true` turns "nobody
  recorded this" into "this is fine", which is the one direction that gets a studio sued.

  **Unknown is a value, never an absence that reads as permission.** An asset with no sidecar row reads
  `unknown, source not recorded`; an asset whose sidecar wrote an empty licence is *recorded* as empty. Both answer
  `None`, and the two stay distinguishable.

  **A folder-level `LICENSE` or `README` is deliberately never consulted.** An inherited licence that looks
  authoritative is worse than an explicit unknown — the pitfall #14 names — so only a per-file record is read, and a
  pack whose README says "all CC0" gets unknown provenance for every file.

  Sidecar CSV columns are matched **by name** (`file`/`path`, `licence`/`license`, `source`, `author`/`creator`), so a
  pack that orders them differently still reads. Two rows for one file is a **conflict**, not last-write-wins.

  **Stored as a trailing optional section, and carried on every result.** `Index.provenance` is written inside the
  **checksummed** body, so a hand-edited licence is a corrupt file rather than a silent change to which assets may
  ship. An index with no provenance writes **no section at all**, making the format a strict superset — `MAGIC` does not
  move and every existing file is a valid new one. Fields are escaped, because a pack called `kenney sounds` would
  otherwise split into two fields and a licence of `-` would read as *nothing recorded*: the same conflation the type
  refuses in memory, reintroduced by the file format.

  `rank::Match` gains `provenance` and loses `Copy`, so a result **cannot** be built without one — forgetting to ask is
  a compile error rather than an unlicensed asset presented as unencumbered. Measured blast radius: one `Match` literal
  in the crate and no consumer iterating by value.

### Documented

- **Incremental indexing: embed only what changed.** `semantic::incremental::update(root, index, embed)` takes the
  index a previous run wrote, scans the folder, and embeds **only the adds** — dropping only the removals and reporting
  `added` / `removed` / `unchanged` / `failed` as separate numbers. Measured: adding one file to a **100-file** library
  makes **one** call to the embedder, asserted on the call count so re-embedding everything and discarding the extras
  cannot pass.

  **Path is not identity, and the index proves it.** `Index::records` is `hash → vector` and stores no paths, so a
  rename is indistinguishable from a file that did not move: it costs nothing **by construction**, with no case that can
  be implemented wrong. A re-encode is a **new asset** — the bytes changed, so the hash did — and that is asserted
  rather than assumed, because the crate cannot tell "re-encoded" from "a different take" and the opposite guess would
  leave the index describing audio that is gone.

  **A file that will not embed is a named failure, never silence.** The first draft iterated `library.assets` and
  ignored `library.skipped`, so a corrupt file that *is* audio by extension appeared in no count at all —
  `an_undecodable_file_is_failed_not_counted` caught it. `indexer::build` already split those skips by meaning; the
  incremental path now does too.

  `update` is a pure function and never becomes a second writer: `indexer::write` owns the file, so a caller composes
  them and puts its own lock around the write. Refs #16.

### Documented

- **ADR 0003: acoustic features, and which crate computes them** — `docs/adr/0003-acoustic-features.md` decides the
  layer the ensemble filters on, and unblocks #21–#25. **All six features are Rust; no sidecar is built.** The
  load-bearing fact is that **`ebur128` 0.1.11 is pure Rust** — nine `src/*.rs`, its C confined to `capi-test/` and
  `tests/c/`, and its `build.rs` gated on the `c-tests` feature, off by default. Verified by building it with
  `default-features = false`: **1.66 s, no `cc`, no C object**. It costs exactly one entry in `ALLOWED_DEPS`, which
  belongs to the commit that declares it, not to this one.

  **Measured, and the measurement changed the decision.** On two long clips `ebur128` agrees with `ffmpeg`'s reference
  to **0.01 LU** (−23.01 vs −23.0; −8.92 vs −8.9) and true peak to 0.02 dB. On a **0.2 s** clip the two disagree
  entirely, and neither is usable: ffmpeg reports **−70.0 LUFS** — the standard's floor presented as a level — while
  `ebur128` returns **`-inf`** from `gated_loudness_multiple`'s `NoBlocksAboveThreshold` arm. `relative_threshold()`
  returns exactly −70.0 when there are no blocks, the same number a genuine −70 LUFS passage gives, so **#21 must carry
  a gating-block count** rather than trust `loudness_global()`. That requirement is now in the ADR instead of being
  rediscovered in #21.

  Tempo ships in Rust with a **±2 BPM floor at 80–160 BPM** (**design default**) that is a test, not a claim, and is
  removed from the ranking if unmet; **key is out of scope** for the Rust path rather than shipped at an accuracy no
  source supports.

- **The shared stream is spent by a play whose pitch range is exact.** `answer` computes
  `cue.pitch.at(choices.rng.next_f32())` before it knows the range cannot use the number, so a `PitchRange::exact(n)`
  cue draws anyway — and every later pick for every *other* cue moves. A project's soundscape is therefore a function of
  its seed and its **play order**, not its seed alone. Measured in
  `tests/play.rs::an_exact_pitch_range_still_draws_from_the_stream`, which pins the current coupling so a fix is
  detectable. Skipping the draw would re-roll every project's soundscape in the build that shipped it, so it is
  `docs/loop/needs-decision/exact-pitch-draw.md`, not a patch.

### Added

- **`SirenMix::peak_db(bus) -> Option<f32>`** — the worst-case peak, or `None` when nothing is on the bus.
  `worst_peak_db` answers `SILENT_DB` for a bus with **no voices**, which is the same number it answers for a bus **mixed
  to the floor** — while its own doc says *"nothing playing is not `SILENT_DB` — it is no peak at all"*. Measured:
  `assert_ne!` between the two fails at `−80.0`. A meter could not tell them apart, and `examples/mixer` filtered
  `voices_on(bus) > 0` before asking — the workaround a collision forces on every caller. `peak_db` removes it, and the
  example now uses it. `worst_peak_db` is unchanged: its one-number form is what a caller wanting a value for every bus
  already has, and changing its signature would break consumers of 0.1.0.

### Fixed

- **The audio tower's embedding now matches the model it targets.** Feeding this crate's own spectrogram to
  `laion/clap-htsat-fused` produced an embedding at **cosine 0.850** against `transformers`'; it now produces
  **0.999999995**. Two constants, both read from `transformers/audio_utils.py`:
  - **The right-hand reflect pad is `WINDOW/2`.** `np.pad` pads `(512, 512)`, and the left pad *shifts* the last frame,
    so the right side needs 512 samples, not the distance to where the last frame's window ends. The old value reflected
    1024 and, before that, read past the buffer — which in a release build is not a panic but whatever the allocator
    held, measured as the last frames of a 1001-frame spectrogram collapsing to the −100 dB floor.
  - **The FFT accumulates in `f64`.** The reference promotes with `waveform.astype(np.float64)` before `rfft`; `f32`
    left the worst spectral value **11 dB** out at the noise floor. With both fixed, the maximum deviation over
    1001 × 64 mel values is **0.0545 dB**.

  The graph itself was already exact — fed the reference's mel it reproduces the reference embedding at cosine
  **1.000000007** — so the entire residual was the spectrogram. Measured in
  `src/semantic/mel.rs::against_the_model::{the_tiled_model_window_matches_the_extractors_own_output,
  no_frame_of_the_tiled_window_falls_to_the_floor}`, which name frames **0, 500 and 1000** rather than checking a shape;
  frame 1000 is the one a short pad turns into −100 dB.
- **`is_longer` is a `bool` tensor.** The graph declares `tensor(bool)` and rejected the `int64` the crate sent:
  *"Unexpected input data type. Actual: (tensor(int64)) , expected: (tensor(bool))"*.
- **Every semantic error implements `std::error::Error`**, all twelve of them. A caller writing `?` in a function
  returning `Box<dyn Error>` could not use any of them — `error[E0277]: MelProblem: std::error::Error is not satisfied`.

- **A `MusicLayers` change naming a layer the score does not have was silently dropped.** `move_layers` applied each target
  with `if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name)`, so a name that matched nothing vanished:
  no log line, no refusal, no counter — the one message in the crate with no failure path. A typo in a layer name was
  therefore a change that did nothing and said nothing. Measured: `MusicLayers::new(vec![("nope", -6.0)])` left the log
  holding only the layers' own `played` lines. The lookup is now a `let … else` that writes
  ``refused `nope`: this score has no layer `nope`, so the change names nothing`` and carries on, so **one bad target does
  not cost the others** — a message naming a real layer and a typo moves the real one and refuses the typo.
  `SIREN_LAYER=typo` captures it.


- **A stinger at `duck_db: 0.0` logged a duck it did not make.** `arm_stinger_ducks` armed `DuckClock::manual` and pushed
  `Entry::Ducked { by_db }` unconditionally, while `model::active_duck_db` returns `0.0` for any `by_db >= 0.0` — so a zero
  depth (or a positive one, meant as a lift) wrote a line saying the music stepped back and the music did not move.
  Measured: `Stinger { duck_db: 0.0, .. }` produced `[Ducked { by_db: 0.0 }]` with `Music` unmoved. The duck is now armed
  and logged only when it can lower the bus, using the same `by_db < 0.0` test the envelope itself uses. A stinger's own
  hit still plays; only the dip is conditional. Iteration 52 fixed the same inert path for a table rule, where a report
  could reach it — a stinger is a message, so the log was the only place a person could see it and it said the opposite.


- **`SirenSounds` never released a sound key, and its handles kept whole audio files resident.** The map exposed
  `insert`/`get`/`len`/`keys`/`load_table`/`missing`/`failed` and **no remover**, and `load_table` **merges** — it asks for
  the keys it has no handle for and never drops one the table has lost. Measured: two tables of ten different keys loaded
  in turn left **twenty** keys held. The map overhead is small (~62 KiB per thousand) but a `Handle<AudioSource>` is what
  keeps an asset alive, and `AudioSource` is `bytes: Arc<[u8]>` — **the whole encoded file** — so every stale key was an
  audio file held for the life of the process. `SirenSounds::retain_keys(&table)` now drops the keys the table no longer
  names; call it before `load_table` on a reload. Not a wrong-output bug: a stale key is never read, checked at every
  `sounds.get`.

### Added

- **`SirenSounds::retain_keys`** — drop the keys a table no longer names. Additive; `load_table`'s contract is unchanged.


- **A cue the table no longer had kept its `VariantPicker` for the life of the process.** `SirenChoices::pickers` is keyed
  by cue name and an entry is created on a cue's first play; nothing removed one. `reset_histories` clears each picker's
  *contents* and is not a retain, so it did not help. Measured: 200 cue names played, the table cut to ten, and **200
  pickers still held** — each with an owned copy of its recent sound keys. `SirenChoices::retain_cues(&table)` now drops
  the pickers of cues the table has lost, and `play_requests` calls it **only on a frame where the table changed**.
  Variant streams are byte-identical to before: a cue the table keeps keeps its history.

### Added

- **`SirenChoices::retain_cues` and `SirenChoices::picker_count`** — the retain, and the number it holds. Additive.


- **A duck rule the mix table dropped kept its start for the life of the process.** `DuckClock::started` is keyed by
  `(source, target)` and was only ever emptied by `clear()`, which nothing calls; `prune(now_s)` retained the *manual*
  envelope list and not the rules'. Measured: **2000 `fire`s followed by a `prune` at t=1000 still held 2000 entries**,
  and 100 table edits left 100 — reachable in the Cauldron, which rewrites and re-reads `family/bevy_siren.ron` on every
  edit. `DuckClock::retain_rules(&settings)` now drops a start no current rule names, and `update_mix` calls it every
  frame beside `prune`. A running duck is unchanged across a retain (−8.0 dB at the same moment before and after), and
  the call costs **13 ns per frame**, measured at 1, 10 and 1000 held starts.

### Added

- **`DuckClock::retain_rules` and `DuckClock::rule_starts`** — the retain, and the number it holds, so a panel can draw
  it and a test can assert it. Additive; `prune`'s signature is unchanged.


- **`MusicScore::default()` was not the score the file loader gives.** `beats_per_bar` carried
  `serde(default = "MusicScore::default_beats_per_bar")` — **4.0** — while `derive(Default)` gave it **0.0**, so the same
  score was four beats and a `Some(2.0)` bar from RON and **zero beats with `bar_s() == None`** from `Default`: no beat
  grid at all, every `NextBar` and `NextBeat` change landing on the frame it was asked for, and
  `problems()` reporting the crate's own default as *"beats_per_bar is not a positive number"*. `Default` is now written
  by hand and returns `default_beats_per_bar()`, so the two spellings of the field are one number.
  `tests/music.rs::a_default_score_quantises_to_the_bar` drives a `NextBar` change from a `Default` score and asserts it
  lands at 2.0 s; it fails on the old tree.
- **A duck at `by_db: 0.0` lowered nothing and said nothing.** `active_duck_db` returns `0.0` for any `by_db >= 0.0`
  and the mix keeps a reduction only `if db < deepest` with `deepest` starting at `0.0`, so such a rule fired, reached
  the log, held for its whole `hold_s` and left the bus exactly where it was — while `SirenSettings::problems()` answered
  `[]`. A `by_db` above zero was already reported as a raise, and a non-finite one cannot be written in RON at all
  (`by_db: nan` answers `Expected float`), so zero was the one depth both authorable and silent. The report now says so,
  naming `by_db` rather than `hold_s`, because for a zero depth with a real envelope the depth is the wrong field. The
  rule's behaviour is unchanged: it still fires and is still logged. `SIREN_MIX=zero` captures it.


- **A cue's `AvoidRepeat` window was frozen at its first play.** `SirenChoices::pick` builds a cue's `VariantPicker` on
  its first play and reused it for the app's life, while `pick_from` took `avoid` fresh on every call — and `remember`
  bounded the window by the deque's **allocator** capacity, not by a number anyone chose. Measured: four keys at
  `avoid: 3` on a picker built at `capacity 1` produced the **byte-identical** 24-pick sequence to `avoid: 1` —
  `adbcbcabcacabcabcbabcbcb` where `avoid: 3` requires `adbcadbc…`. Sixteen of twenty picks returned within three of
  themselves. `VariantPicker` now keeps an explicit `window`, **grows to meet the `avoid` passed to `pick_from` and
  `pick`**, and `Default` remembers one pick instead of none. A table an editor reloads with a larger `avoid` takes
  effect on the next play. Reachable in the Cauldron, whose audio panel rewrites and re-reads `family/bevy_siren.ron`;
  `reset_histories` had no caller in the crate, its tests, or the panel, so nothing else cleared it.

### Added

- **`VariantPicker::set_capacity` and `VariantPicker::capacity`** — grow or shrink the remembered window, and read it.
  Additive, so no consumer breaks.
- **`SIREN_MIX=zero`** in `examples/mixer` — a table whose only duck is `by_db: 0.0` with a real hold and fade, so the
  panel shows the zero-depth sentence as its whole `Problems` block. `SIREN_MIX=broken` already fills the block's
  three-sentence cap, and a fault added there would be the invisible `and 1 more`.
- **`examples/render` writes each cue's variant stream** as a `variants <cue>:` line — the rule, the sound count, the
  twelve picks and the longest run. Variant selection is audible and was otherwise only inferable from a wav.

- **`examples/render`'s `mix.wav` was written past full scale.** It applied `sum_db` of **every** cue's gain — `+18.58 dB`
  — to one 0.5-amplitude tone, so the file was `4.2×` full scale: `ebur128` read **`+0.2 dBFS` true peak, `−4.3 LUFS`**.
  `sum_db` of ten cues is the number if all of them sounded at once through nothing; the file's job is to let the mix be
  heard, so it is now written at `settings.master_db` and reads **`−6.1 dBFS`, `−18.6 LUFS`** (below the −14 a game mix
  targets). The `mix:` line states what the sum is and what the file is. Example only.
- **`examples/mixer`'s own table no longer clips.** `Effects` reached `+6.0 dBFS` and `Music` `+4.0 dBFS` with two
  voices, which the panel has shown since iteration 49 and which the guide's new headroom section makes actionable:
  `Effects` `0.0 → −7.0` and `Music` `−2.0 → −7.1`, both a decibel inside the rail (`2 × 10^(−7/20) = −0.98 dBFS`). The
  panel's peak clause disappears because nothing reaches 0 dBFS. Example only.

### Added

- **The guide states that `master_db` is the headroom trim, and that it is inside `worst_peak_db`.** The decision file
  `docs/loop/needs-decision/above-unity-gain.md` recommends *no* report for loudness and names the master as *"the tool
  for it"*; nothing had tested that sentence or written it where a reader looks. `tests/mix.rs::
  the_master_is_inside_the_worst_peak` measures it — one voice at unity reads `0`, `−6`, `−20` dBFS at masters of `0`,
  `−6`, `−20` — and `docs/guide/buses.md` now says so, with why `problems()` stays deliberately silent about loudness.


- **`SirenMix::worst_peak_db(bus)` — the worst-case peak a bus can reach this frame, in dBFS.** The crate could mix
  past full scale and had no way for a game to ask: `max_voices` is a **sample cost** (*"fewer than a mixer's worth of
  sample cost"*, `docs/research/siren.md`), never a headroom guarantee, and nothing combined a bus's gain with its live
  voice count — though `model::sum_db` already held the arithmetic R6 states (*"several sounds **sum in linear**"*).
  Eight sources at 0 dB sum to **+18.06 dB**; two on a −6 dB bus reach **+0.02 dBFS**. It is an **upper bound** — the
  crate mixes samples and never reads their amplitude — and reads `SILENT_DB` for a bus with no voices. Additive: a new
  method, no field or variant. The shipped example's own table reaches **+6.0 dBFS** on `Effects` and **+4.0 dBFS** on
  `Music` with two voices, which the panel now shows.

### Fixed

- **`examples/render` no longer calls clipping "the budget's job".** Its line read *"307 samples would clip (the budget's
  job, not the gain's)"* — `voices-8.wav` has **307 samples at the rail**, measured from the WAV, and the budget is a
  sample cost that never claimed to bound the sum. It now says so and names `SirenMix::worst_peak_db`.


- **A bus asking for more voices than the table allows is reported.** `SirenSettings::bus_voice_limit` is
  `min(cap, total)`, so `BusLevel::new("Effects").max_voices(16)` under `max_voices: 4` mixes **four** — and neither
  `BusLevel::max_voices`'s doc (which names only the `0` sentinel) nor the guide's *"a bus's own `max_voices` caps it
  further"* said that a **larger** number does nothing. `problems()` gains a twelfth sentence —
  *"bus `Effects` asks for 16 voices but the table's max_voices is 4, so it mixes 4"* — additive, since the report is
  `Vec<String>`. A bus at `0` (the sentinel) and a cap **equal** to the table's stay silent; both are asserted. The
  behaviour is unchanged: the bus still mixes the table's four.


- **A duck that starts on the bus it lowers is reported.** `Duck::new("Effects", "Effects")` is accepted and
  **quietens its own voices**: `fire_all` matches rules on `source`, `duck_db` matches them on `target`, and for a
  self-duck one bus is both — so a footstep pulls down every other effect for the rule's whole envelope, and itself with
  them. Measured: `-6.0 dB` on the frame its own voice starts, and the bus mixed that much lower. `problems()` gains an
  eleventh sentence — *"the duck on `Effects` starts on the bus it lowers, so every voice on it quietens itself"* — which
  states the mechanism rather than forbidding it, because a self-sidechain is a real device. Additive: the report is
  `Vec<String>`. The behaviour is unchanged.


- **A duck with neither a hold nor a fade is reported.** `Duck::new(source, target)` leaves `hold_s: 0.0` and
  `fade_s: 0.0`, and `active_duck_db` returns `0.0` past the hold when `fade <= 0.0` — so the shortest rule a project
  can write **fires, is recorded in the log, and is inaudible on the very frame it fires**. Measured:
  `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time` reads `0.0` from the bare rule and `-8.0` from the
  same rule with `.hold_s(0.25).fade_s(0.6)`. The crate's own `default_ducks()` chains both, which is why its pair
  works and why nothing said otherwise. `SirenSettings::problems` gains a tenth sentence —
  *"the duck on `Music` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s"* — the
  report is `Vec<String>`, so this is additive. The **behaviour is unchanged**: a project that relies on an inert rule
  today still gets one, it is simply named now. A raising rule is not reported twice (the guard is `by_db < 0.0`).


- **The panel's `Problems` block draws the mix table's faults, not only the cue table's.** It read
  `table.problems_in(&settings)` alone, so a mix table with `max_voices: 0`, a NaN `master_db` or a bus listed twice
  drew `Problems: none` **while every bus read −80 dB** — the report said a table it could not run was fine. It now
  concatenates `SirenSettings::problems()` with the cue table's, and `SIREN_MIX=broken` produces the former so the block
  can be seen doing it. Example only; no `src/` behaviour changed.

- **The panel's cue list is bounded at eight rows.** `SIREN_FAULTS=many`'s fourteen cues drew twelve rows and pushed
  the `Problems` block and then the whole log off a 1280×720 capture — measured, the log's heading was not on screen and
  the lowest ink sat at `y = 719` of 719. It now says `and N more; the log below is what played`, the same rule the
  block under it already followed, and the log's line count subtracts that row as well as the block's. Margins after:
  9 px clean, 10 px broken, 10 px with fourteen cues.

- **`SirenSettings::problems`'s doc lists all nine checks it performs.** It named six — `master_db` not a number, a
  bus's gain not a number, and a duck's hold or fade were unlisted — while `MusicScore::problems`'s doc has listed all
  nine since iteration 34. Four of the nine sentences had **no test at all**; `tests/mix.rs::problems_names_every_mistake`
  now walks all nine, each shown to be its own fault, and each of the four reddens it when removed. No behaviour changed.

### Added

- **`MusicState::blocked(layer)` reports a virtual voice.** A score may name more layers than the budget has room for;
  when the layer's `Play` is refused the plugin records the live count and asks again when it falls below. That state
  was private, so a panel drawing only `layer_db` printed `-80.0 dB` for a layer **waiting for a voice** and `-80.0 dB`
  for one authored silent — the same string for two different answers to *"why is the music quiet"*. The accessor
  returns `Some(n)` (the count it waits for) or `None`, and is additive: no variant, field or exhaustive `match`
  changes. Gregory §14.5.8 — *"a virtual voice is a sound the game believes is playing but that has no hardware
  voice"*. Proved by `tests/music.rs::a_refused_layer_start_is_tried_again_when_room_frees`, whose mutation (returning
  `None`) reddens it.

### Fixed

- **A sound whose owner is despawned now says it stopped.** `Play::on(entity)` parents the voice with `ChildOf`, so
  despawning that entity — the ordinary end of a sound on a vehicle or a torch — takes the voice with it. Neither
  backend logged it: the crate's own mixer freed the slot in `start_pending` since iteration 7 and wrote **no line**,
  and on Bevy's path the voice entity is simply gone, so `retire_voices` — which runs during `Mix`, one frame *before*
  Bevy applies the despawn — never sees it. The log's last word about such a sound stayed `played` while the mix counted
  one fewer voice, against `"ended"`, `"stop"`, `"cue stopped"` and `"stolen by …"` for every other end.
  `mix::log_despawned_voices` records each live voice's cue and whether it had a `ChildOf`, reads
  `RemovedComponents<Voice>`, and writes one `stopped \`<cue>\`: owner despawned` for a parented voice that vanished —
  on **both** backends, once. A game that despawns a voice it never parented still logs nothing. No public type,
  variant or field changed. Proved by `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` (no feature flags,
  so the default backend) and `tests/mixer.rs::a_despawned_voices_mixer_slot_is_freed` (which also pins that the line is written once).

### Added

- **`examples/mixer` shows a music layer waiting for a voice.** The `Music` block appends `waiting` to a layer's gain
  when the budget could not give it a voice — `combat -80.0 dB waiting` — and `SIREN_TIGHT=1` gives the `Music` bus one
  voice so the state occurs in a capture (`docs/media/mixer-waiting.png`). The cap is on the bus, not the table, so the
  other buses keep their room and the picture isolates the point.
- **`examples/mixer` shows a parented sound ending.** A `parent` step spawns an owner, plays `engine.idle` on it with
  `Play::on`, and despawns the owner two frames later — so the panel's log now carries
  `0.68s stopped \`engine.idle\`: owner despawned` beside `Effects 1 / 2 voices`. The script is now
  `steal,spatial,parent,layer,duck,stinger,poly,refuse`.

### Changed

- **A steal says which cap took the voice.** `Entry::Stopped`'s reason was `stolen by footstep.grass` for **three
  different budgets** — the cue's `max_instances`, its bus's `max_voices`, and the table's `max_voices` — so an author
  who saw it had no way to know which number to raise, and a step that took three voices logged two identical lines.
  It now ends `(cue)`, `(bus)` or `(table)`: `stolen by sword.hit (bus)`. `room_for` returns which cap chose each victim.
  Every existing `starts_with("stolen by")` check still matches. Proved by
  `tests/mix.rs::a_steal_names_the_cap_that_took_the_voice` (forces the bus cap) and a `(cue)` assertion in
  `a_cue_respects_its_own_polyphony`; the mutation — one format string without the cap — reddens both.

### Added

- **`examples/mixer`'s cue row shows how many of a cue may sound.** The list drew name, bus, mode, gain and `priority` —
  and `priority` decides who *survives* a full budget, while `max_instances` decides whether the next play may start at
  all. `footstep.grass` admits two (`x2`), `ui.click` one (`x1`), and the documented `max_instances: 0` reads `all` and
  still pairs with `UnlimitedInstances` in `Problems` rather than inventing a number. A new `poly` step asks a cue three
  times in one frame, so the row's `x2` is proved by two live voices and one `(cue)` steal — and it runs last in the
  default script, so the still's log keeps the stinger, the dialogue duck and the spatial play above it.

- **A played gain is written to two decimals.** `Entry::line` was
  ``"{at_s:.2}s played `{cue}` ({sound}) on {bus} at {gain_db:.1} dB"``. One decimal cannot show a distance law: the
  default `Attenuation::Inverse { near_m: 4, rolloff: 1 }` gives **−6.0206 dB** at 8 m — `cues.yaml` case 14's number —
  and `-6.0` is also what a cue authored `gain_db: -6.0` produces, so the log could not tell a distance from a fader and
  disagreed with the panel's own cue row, which prints two. `gain_db`'s *field* keeps full precision; only the sentence
  rounds. **A host that parsed the log's one-decimal gain gets two decimals.** A duck's `by_db` stays at one decimal: it
  is an authored depth, not a computed product. Proved by
  `tests/model.rs::a_played_gain_is_written_to_two_decimals`, whose mutation (`{:.1}`) reddens it.

### Added

- **`examples/mixer` shows a placed sound.** The panel had **no spatial cue and no `.at(..)`** — grep found zero of
  either — so the crate's spatial path (three attenuation models, a panner, and `Refusal::SpatiallyMute`) was tested and
  never drawn. The table gains `bang` (`spatial: true`, `Effects`, priority 30), a `spatial` step plays it at **8 m**,
  and its cue row carries the law and the gain that distance gives — `inverse at 8 m -6.02 dB` — computed with
  `model::distance_gain`, the same function a play uses. The log's other numbers are unchanged. The log also shows **six**
  lines rather than seven: the eighth cue row costs 21 px, and with seven the lowest ink sat at `y = 719` of 719 — no
  margin, a clipped glyph.

### Fixed

- **The audio panel names the duck whose depth it shows.** A bus row printed `SirenMix::ducked`'s depth — the **minimum**
  over every active rule — beside the **newest** `Ducked` entry's `by`. With one duck those are the same rule; a stinger
  firing over a line of dialogue puts two on one bus, and the row read `duck -8.0 dB by stinger` when −8.0 belonged to
  `dialogue.greeting`. Two numbers from different sources, presented as one fact. The row now names the newest entry with
  **its own** `by_db` when the two differ, and the plain `by <cue>` when one rule explains the number — including a rule
  that is recovering, which is not a second cause. Example only; no `src/` file changed.

### Added

- **The panel plays the stinger.** `music.stinger` had been in `examples/mixer`'s table since it was written and **no
  step ever wrote a `Stinger`**, so the crate's second music feature and the one duck whose cause is a mechanism
  (`by: "stinger"`) had never appeared on screen. `SIREN_KEYS=…,stinger,…` (in the default script before `refuse`) fires
  it, and `docs/media/mixer.png` is now captured on that frame. No `src/` file changed.

### Changed

- **`max_instances` is documented as a budget on plays, not a standing count.** The field read *"how many of this cue
  may sound at once"*; the crate keeps it at the moment of each play, so a table edited while voices are sounding leaves
  them playing and refuses the next request rather than cutting a voice the game did not stop. `Cue::max_instances`,
  `CueTable::insert` and the buses guide now say which reading the crate takes, and two tests hold it — including the
  mutation that would adopt the other one. No behaviour changed.
- **`UNSTEALABLE_PRIORITY`'s rule is now stated directly in the suite.** It was proved through
  `the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue — a path that takes a
  paragraph to explain. Two tests now ask it the way the rule is written: a priority-0 request against a priority-200
  voice is refused with nothing stolen, and a stealable voice still plays beside one so neither can pass vacuously. No
  `src/` file changed; the mutation check (`steal` ignoring `unstealable`) reddens both new tests **and** the old one.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- **`CueTable::problems_in(&SirenSettings)`** — the report against **the buses the mix actually has**. `problems()` is a
  wrapper over `problems_with(&DEFAULT_BUSES)`, so a project that lists its own buses — fully supported, and the answer
  `answer`'s `UnknownBus` check, `update_mix` and `SirenSettings::problems()` all read — had every cue on a custom bus
  reported as `UnknownBus` while the crate accepted and mixed it. `problems_in` reads `settings.bus_names()`, the same
  answer every other reader uses. `problems()` keeps its documented default-four meaning and now says what it assumes.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`SirenSettings::problems()` agrees with the mix about what a bus is, and names each fault once.** The report built
  its own bus set from `self.buses` instead of asking `bus_names()`, the one answer every other reader uses — so a table
  with `buses: []`, which `docs/guide/buses.md` documents as *"the crate's four … everywhere"*, was reported as having
  **four missing buses** while the crate mixed it correctly. And a duck fault was pushed once per rule, so the crate's
  own default pair (`Dialogue → Music`, `Dialogue → Effects`) printed the same sentence twice — a panel reading
  `problems()` verbatim read one fault as two. The report now asks `bus_names()`, and deduplicates by **sentence**: two
  rules that are the same fault are one, while two different faults about one bus (a duck starting on it, a duck lowering
  it) stay two. No public type changed.
- **A music layer's gain that is not a number no longer silences the stem behind a `NaN` fader.** Iteration 18 gave the
  crate one rule for a non-finite caller value — the neutral value for what it measures — and applied it to the numbers
  that enter `answer`. A layer's `gain_db` entered by another door: `NaN` went into `layer_db`, `target_db` and the
  voice's `base_db`, `chain_db` silenced the sink (correctly), and **`MusicScore::problems` said nothing**. The layer is
  now normalised with the same `model::finite_or` in `start_layers` and in every `MusicLayers` target, so a layer with a
  bad gain starts at `0.0` dB and the fader agrees with the sound; `SILENT_DB` is finite and is untouched, so a score
  that starts a layer silent still does. `problems()` gains one sentence, `"music layer \`calm\` has a gain that is not
  a number"` — the score's report is `Vec<String>`, so no public enum changed.
- **A gain or detune that is not a number is the neutral value, not silence — and `Played` reports the rate the device
  is at.** `chain_db` returns `SILENT_DB` for a non-finite part, which is right for a bus level the crate computed and
  wrong for a number an author typed: a cue with `gain_db: NaN` (which a RON file can contain) **played silent** while
  `Played` reported `-80.00` as though that level were authored, and nothing said why — which R6 forbids (*"a play is
  never silent: `Played` or `Refused`, both with the reason"*). `semitones_to_ratio` already played a non-finite detune
  at `1.0x`, but `Played.pitch` carried the raw `NaN`. One rule now covers every non-finite caller value, matching what
  `distance_gain` already does for a distance (`docs/guide/spatial.md`): no gain, no detune, and `Played.gain_db` /
  `Played.pitch` report the numbers the voice is **actually at** — a detune past the clamp reports `±48` semitones,
  which is the rate that plays. `chain_db`'s silencing rule is untouched, so a silent bus still silences its voices. The
  stricter answers (a `CueProblem::BadGain`, a `Refusal` variant) are recorded as
  `docs/loop/needs-decision/bad-gain-problem.md`: both public enums are exhaustive and a new variant breaks every
  consumer's `match`.
- **`Backend(true)` with no `Output` no longer gives a voice nobody plays.** `Backend` is a public resource whose flag a
  host may set, and `spawn_voice` decided *"the mixer owns playback"* from that flag alone: with the flag set and no
  `Output`, a voice carried **no `AudioPlayer`** (the mixer owns it) and had **no mixer** (none exists), so it could not
  sound — and a voice that cannot sound never ends, so it was never retired and held its budget slot for the life of the
  process while `Played` said it had played. Ownership is now `backend.0 && output.is_some()`, which is what
  `start_pending`'s doc already promised (*"without an `Output` … Bevy's `AudioPlayer` owns every voice"*). A world with
  a mixer is unchanged: its voices still carry no `AudioPlayer`, so the two paths cannot both play one request.
- **A music layer naming a cue that ends no longer machine-guns the play path.** `MusicScore::problems` states the rule
  — *"names cue `hit`, which is not a loop, so it cannot hold a layer"* — and the plugin ignored it: a `Mode::Once` cue
  was started, its voice was retired (a once voice **ends**; `retire_voices` only skips loops), and the layer was asked
  for again **every frame** — 10 plays in 10 frames, with no `Refused` and no log line, while an unknown cue was refused
  with a reason. `start_layers` now reads the cue the layer names this frame, does not start a cue that is not a lock
  `Mode::Loop`, keeps the layer in `names()`, and writes **one** line in `problems()`' own words. A table edit that
  makes the cue a loop starts the layer on the next frame. A `Refusal` variant for the same fact is recorded as
  `docs/loop/needs-decision/not-a-loop-refusal.md` — it is public API and waits for the owner.
- **A music layer's voice could die and the layer never notice.** `MusicState` wrote the entity a layer's `Played`
  reported and never reconciled it, so a voice stopped by the game (`StopCue`), taken by the budget, or refused at its
  first play left `voice(name)` naming a **despawned** entity, `live()` counting it, and the stem silent for the rest of
  the process. The mirror now reconciles every frame: a dead voice is cleared, a stem that died behind the plugin's back
  is restarted at its own gain, a `StopCue` stays stopped, and a refused first start is retried **when a voice frees**
  (one refusal line, not one per frame).
- **A music layer is unstealable whatever priority its cue authored.** `Voice::unstealable`'s doc says *"music, stingers
  and dialogue are not stolen"*, but the mechanism was `priority >= UNSTEALABLE_PRIORITY`: a layer authored with the
  crate's default `priority: 0` was called music and treated as a footstep, so a `ui.click` on the same bus at
  `max_voices: 1` stopped the stem. The budget now reads the `MusicVoice` the plugin already inserts; the newcomer is
  refused instead.
- **A `MusicScore` edited at runtime now reconciles its voices, not only its layer list.** `MusicScore` is a resource a
  project and the Cauldron edit, and `start_layers` only ever *added* layers: a layer **removed** from the score kept a
  looping voice playing forever, ownerless (`retire_voices` skips loops by design), and a layer **retargeted** to a new
  `cue` kept the old sample while the new one never started. Both disagreed with the score in silence. A removed or
  retargeted layer's voice is now marked `Stopping` with the true reason (`"layer removed"` / `"layer retargeted"`), so
  it fades over `DEFAULT_FADE_S` and the log says why; a retargeted layer starts its new cue the same frame. A **gain**
  edit still keeps the voice — that is what `move_layers` moves. `MusicState::names()` keeps its documented score order.
- **A `StopVoice` for a voice that has already ended no longer panics.** `stop_requests` took the entity straight from
  the message and issued a command on it; `retire_voices` despawns a one-shot the frame it ends, and `Played::entity` is
  documented as the handle a game keeps for `StopVoice` — so holding it one frame too long hit Bevy 0.20's default error
  handler, which **panics** (`Entity despawned … its index now has generation 1`). The request is now resolved against
  the live voice query first: there is nothing left to stop, so nothing happens and nothing is logged. The same guard
  makes a second `StopVoice` for a voice already fading a no-op, so a fade is begun once.
- **An empty `buses` list mixed every voice at −80 dB.** `answer`, the editor's panel and `SirenSettings::bus_names()`
  all treat a table listing no buses as the crate's four, but `update_mix` iterated `settings.buses` directly — so with
  `buses: []` a cue on `Effects` was **accepted and then mixed at −80 dB**, with no refusal and no log line. The mix now
  iterates the same resolved list `bus_names()` returns.
- **`SirenMix` no longer keeps a bus that left the table.** `db` and `ducked` were written by insertion and never pruned,
  so a project that swapped `SirenSettings` kept every bus from the previous table at its last level: `bus_db` answered
  for a bus that did not exist, `deepest_duck()` named one forever, and a removed bus's duck was frozen at its deepest.
  Both maps are now pruned to this frame's buses before the bus loop, which leaves the previous value readable for a bus
  that stayed — so a fader still eases rather than steps.
- **A stinger that cannot play no longer ducks the music.** `play_stingers` wrote the `Play` and armed its duck in the
  same breath, but `Play` is answered later in the frame — so a refused stinger left the `Music` bus down for a sound
  nobody heard, and `SirenLog` stated the duck **and** the refusal on adjacent lines. A new `arm_stinger_ducks` arms
  the duck when `Played` arrives, after `Play` and before `Mix`, so the dip still lands on the frame the stinger fires.
  Stinger refusals now write a `Refused` message as well as a log line, and a cue **not in the table** is refused as
  `UnknownCue` instead of inheriting `Music` through `unwrap_or`. A wrong-bus stinger has no `Refusal` variant that is
  true, so its sentence stays in the log: `docs/loop/needs-decision/stinger-refusal.md`.
- **A non-spatial cue given a position is no longer routed through the two-ear panner.** `spawn_voice` decided Bevy's
  `PlaybackSettings.spatial` — and whether to attach a `Transform` — from `request.position.is_some()` alone, so a cue
  authored `spatial: false` and played with `.at(..)` went through `rodio`'s panner, which applies `min(1, 1/d²)` as
  well as a pan: **−38.7 dB at 8 m**, while `Played.gain_db` and the log reported **0.00 dB**. One predicate now
  decides the attenuation, the panner and the `Transform` (`cue.spatial && position.is_some()`). **A project that
  relied on the accident loses a pan it was never promised, and its meter was wrong while it happened.** Refusing such
  a play instead is `docs/loop/needs-decision/non-spatial-position.md`.
- **The crate's own mixer reuses a voice slot instead of retaining every sound ever played.** `MixerBackend` pushed a
  `VoiceTrack` — a `rodio::Player` and its queues — per play and never removed one, so a game on the `mixer` backend
  held one player for every sound it had ever played: 400 one-shots held **400 slots** on a real device. `play` now
  reuses the first slot whose voice has ended, bounding the list by the peak *concurrent* count (6 for those 400), and
  the index it returns packs a slot **and a generation** (`model::pack_voice`), so a `MixerVoice` held by an entity the
  world has not finished despawning addresses nothing rather than the slot's next occupant. `MixerBackend::voices()`
  now reports slots, not sounds played. No public type or trait signature changed.
- **A sample the decoder refuses names the sound key, not the cue.** `start_pending` reported a decode failure as
  `MissingSound { sound: voice.cue }`, putting the cue's name (`footstep.grass`) in a field documented as the key a
  project maps to a file (`step_03`) — so a corrupt `step_03.wav` sent a person looking for a file that never existed.
  It now recovers the key from the same cue and variant `answer` chose. The variant's sentence covers both of its
  causes, *"no handle or undecodable bytes for sound `step_03`"*; a variant that distinguishes them is
  `docs/loop/needs-decision/undecodable-sound.md`. No public type or field changed.
- **A `VoiceLimit` refusal names the bus only when the bus refused.** `room_for` has three hard caps — the cue's
  `max_instances`, its bus's `max_voices`, the table's `max_voices` — and returned one uninformative `None` from all
  of them, so every refusal reported the cue's bus. A probe proved the table case blamed **`Ambience`** for a cue
  playing on `Effects`. It now returns the cap (private `Cap::{Cue, Bus, Table}`): `bus: Some(name)` only for the
  bus's own cap, `bus: None` otherwise, which is what the field's own doc says (*"the bus that had no room, when one
  did"*). **A host that matched `VoiceLimit { bus: Some(_) }` to mean "the limit refused at all" now sees it only for
  the bus case**; `VoiceLimit { .. }` still matches every one. A payload that also names the cap is
  `docs/loop/needs-decision/refusal-names-its-cap.md`.
- **`MusicLayers::fade_s` is the crossfade it says it is.** The field was accepted, stored in `Pending`, and read by
  nothing: `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms and
  nothing said so. `model::tau_for_fade` derives the time constant from `approach`'s own definition
  (`fade_s / ln(1000)`, an arrival within one part in a thousand), and each layer carries the fade it was last given.
  `fade_s(0)` keeps `MIX_TAU_S`, so the builder's default is unchanged.
- **A frame with no elapsed time holds a layer's gain.** `approach` returns its target for `dt <= 0`, which is right
  for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend the whole
  authored fade in one step. `move_layers` now holds the gain when `dt <= 0`.
- **`SirenSettings::seed` is now the stream the crate draws from.** `SirenChoices` was `init_resource`d from a crate
  constant and never rebuilt, so a project's `seed:` was read by nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. `play::seed_choices` replaces the stream (and every variant history) when the mix
  table's seed changes, so two seeds are two streams and one seed replays itself — `cues.yaml` case 17, which could
  not pass before. No field moved: the setting existed and now works.
- **A stinger's `duck_s` is a hold, not a fade.** `play_stingers` passed it as `DuckClock::manual`'s *fade* argument
  with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired: a −9 dB stinger read
  **−5.01 dB** at its midpoint and the music swelled back underneath it. It now holds for `duck_s` and releases over
  `DEFAULT_FADE_S` (0.12 s), reading **−8.99 dB** at the same point. A project-chosen release is
  `docs/loop/needs-decision/stinger-release.md`.
- **A voice that ended now frees its slot on the crate's own mixer.** With the `mixer` feature, a `Mode::Once`
  voice was never retired — the crate asked Bevy's `AudioSink`, which that backend does not create, and nothing ever
  set `Completes` — so it kept its place in the voice budget forever. After `max_voices` (32 by default) one-shots,
  every later play either stole an audible voice (with a full budget, silence was hidden behind a steal) or, for an
  unstealable cue, was refused `VoiceLimit` for the life of the process. `retire_voices` now asks
  `AudioOutput::finished`, the query the backend already implemented, and writes the same `ended` log line Bevy's
  path writes. Gregory §14.5.8.4: a stopped voice *"frees its resources for use by other voices"*.
- **A voice's own gain is written on both backends.** `update_mix` skipped a `MixerVoice` entirely, so a music layer
  moved by `MusicLayers` stayed at the gain it started at, and a stop fade began from silence and despawned on its
  first frame — a cut where the crate promises a fade. The mix now writes the voice's own gain into the mixer and
  its whole chain into a Bevy sink, records which in `Applied`, and one writer still owns both.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- `examples/budget` (`--features mixer`) — a headless run of N one-shots against a voice cap, with a scripted output
  that ends each sample, writing the live count and the refusals per frame as a CSV, a text summary and an SVG plot.
  It is the evidence harness for "an ended voice frees its slot", from `docs/loop/cues.yaml` cases 20 and 21.
- `tests/mixer.rs` — three tests behind the `mixer` feature: a one-shot's slot is freed (fails on the unfixed tree), a
  loop is not retired when its source ends, and a fading voice is not also reported ended.
- `examples/render` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  WAVs and a `render.txt`, so a change can be judged by spectrogram and loudness without an ear. No public API
  changes: it drives `bevy_siren::model` directly.
- `examples/perfcheck` — µs per frame and per voice at 8, 64 and 256 voices.
- `.omp/skills/build-loop` — the loop itself, as a project skill: one iteration from orient to log, with the four
  traps that cost an iteration each written down where the next run reads them.
- `docs/loop/cues.yaml` — the fixed cases every iteration runs, each with its observation and the source for its
  number; `docs/loop/runs/` and `docs/loop/needs-decision/` for a run's evidence and the questions the loop will not
  decide itself. `docs/loop-log.md` becomes `docs/loop/log.md`.
- `docs/loop/needs-decision/remove-completes.md` — whether the public `Completes` component, which cannot ever be
  true, is removed in the next minor version.

## [0.1.0] - 2026-11-02

The first release: sound for Bevy 0.20 as data, mixed in buses.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- **A cue table as data.** `CueTable` of `Cue`s, each with variants, a bus, a gain, a pitch range, a repeat rule, a
  polyphony limit, a priority and an attenuation. Every play answers exactly one `Played` or `Refused`, and a refusal
  names what is missing (an unknown cue, no variants, a missing sound key, an unknown bus, a full budget, no listener).
- **Buses that duck.** `SirenSettings` with `Music`, `Effects`, `Ambience` and `Dialogue` out of the box, per-bus level,
  mute and voice cap, and `Duck` rules that lower one bus under another for a hold and a fade. `SirenMix` is this
  frame's mix — one number per bus, ducks and master included — for a panel or a game's own meter.
- **A voice budget that refuses politely.** Per-cue, per-bus and table-wide caps, with stealing by lowest priority then
  oldest, an `UNSTEALABLE_PRIORITY` above which nothing is taken, and a `Refused { VoiceLimit }` — nothing spawned —
  when the budget cannot be made room in.
- **Layered music on the bar.** `MusicScore` of `MusicLayer`s and a tempo; `MusicLayers` moves their gains with
  `When::{Now, NextBeat, NextBar}`, quantised by `model::next_boundary` from tempo alone. A layer at `SILENT_DB` stays
  in the mix so every layer is phase-aligned; a `Stinger` plays at once and ducks the music.
- **Spatial attenuation.** `Play::at(position)` with `Attenuation::{None, Inverse, Linear}`, Bevy's `SpatialListener` as
  the ears, and the backend panner's own distance law divided back out so a cue's curve is the one a project hears.
- **A log of what played and why.** `SirenLog` keeps the last 256 decisions — played, refused, stopped, ducked — each
  with the real time it happened.
- **Two backends.** Bevy's `AudioPlayer` by default, or the crate's own summing graph on `rodio` behind the `mixer`
  feature, where a bus's gain is one number per bus per frame.
- **The tables as RON**, behind the `serde` feature, with `from_ron`/`to_ron` on `CueTable`, `SirenSettings` and
  `MusicScore`, and a `problems()` on each that returns sentences a person reads.
- `examples/mixer` — the audio panel with its capture knobs, and `examples/music` — a scripted calm → combat → calm
  that checks every transition landed on a bar.

[Unreleased]: https://github.com/bevy-coven/bevy_siren/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bevy-coven/bevy_siren/releases/tag/v0.1.0
