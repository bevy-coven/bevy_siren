# Iteration 110 — research: the playable example (`#38`)

## The item

`#38` — *"An example that indexes a directory, takes a description, and plays the top result through a bus."* Acceptance:
`cargo run --release --example semantic -- <dir> "<description>"` indexes and plays; a run with nothing to run on *"prints what is
missing and exits non-zero"*; the example is listed in `docs/media/README.md` with the command that regenerates its picture.
It carries `required-features = ["semantic"]` because CI runs `cargo build --examples`.

## What I re-checked before believing the old label

My own log said `#38` waited on the editor. The issue says *"Blocked by #33"*, and `#33` is `bevy_grimoire`'s audition panel — an
**ordering in the issue graph**, not a technical dependency: everything in `#38`'s scope is this crate's. The thing that
*did* make it look impossible was *"the real CLAP graphs are not on this machine"*. That was also wrong, in the way `#28` was:

| what I believed | what is true (this machine, measured this iteration) |
|---|---|
| the 501 MB graph is not here | the **weights** are: `~/.cache/huggingface/hub/models--laion--clap-htsat-fused` (50 GB cache; `model.safetensors`, `tokenizer.json`) |
| no way to make an ONNX | `python3` (3.9.6) has `torch 2.8.0`, `transformers 4.57.6`, `onnx 1.19.1`; `/tmp/export_text.py` and `/tmp/export_clap.py` are the iteration-62 scripts that exported both towers |
| `big` has no checkout | irrelevant: this Mac exported them before (the scripts record *"Apple M4, onnxruntime CPUExecutionProvider"*) |

So a **real** run is possible here, which also settles the gap iteration 109's review left open: `siren-tools --model` has only
ever run against a 16 KB fixture.

## Sources

| Source | What it reports |
|---|---|
| Wu et al., *Large-Scale Contrastive Language-Audio Pretraining with Feature Fusion and Keyword-to-Caption Augmentation*, ICASSP 2023, DOI `10.1109/icassp49357.2023.10095969` | the model this crate wraps. **Freesound text→audio mAP@10 is 24.9** for HTSAT-RoBERTa with feature fusion (24.5 without; 22.9 with keyword-to-caption augmentation) — a model whose top-10 is right about a quarter of the time, so a list the user **chooses from**, with the reasons shown, is the honest presentation. Feature fusion *"enables the model to handle variable-length input"*, which is the `is_longer` input of the audio graph |
| this crate's iteration 69 measurement (`docs/guide/semantic.md`, *The floor*) | the correct and incorrect semantic cosine bands **overlap** (0.126–0.429 against −0.104–0.150), so the default floor is `0.0`, and a ranking is shown with its scores rather than filtered |
| ADR 0007 / `#29` | the stated workflow is *"describe, see three, pick one"*: `tools::DEFAULT_K = 3`; each result carries its breakdown and provenance so *"a second call to fetch any of that would be the round trip the issue exists to avoid"* |
| `needs-decision/README.md`, `AGENTS.md` | an example is neither the public API nor the message types nor the definitions-file schema, so it is built, not asked about; the dependency list is closed (`ALLOWED_DEPS`) and an example adds no dependency |
| `examples/music.rs`, `examples/render` | the house style of a terminal example: it **prints what it measured and exits**, so a person can read the check |
| Bevy `0.20.1` local source (`bevy_asset`, `bevy_app`, `bevy_audio`) | read for `AssetPlugin::file_path`, `ScheduleRunnerPlugin` and `AudioPlugin`, below |

**A gap, said plainly:** home-still has nothing on how a sound-library result list should be presented (two searches returned
unrelated papers). The presentation is a **design default**, taken from the issue's own *"describe, see three, pick one"*.

## The technique

The example is **orchestration of calls that already exist and are tested** — it adds no ranking rule and no second play path:

1. `library::scan` → `indexer::build_from_model` (audio tower, `mel::Filterbank::clap()`) → `indexer::write`, or `indexer::read` when
   an index already exists and is not stale;
2. `Embedder::embed` (text tower + vocabulary) → `tools::search_music` — **the same call the MCP tool makes**, so the example and
   `siren-tools` cannot rank differently;
3. `tools::audition` → a `Play` through a Bevy app with `SirenPlugin`: **one play path**, so the voice budget, the bus and the log
   apply, and the example answers `Played` or `Refused` like any game would.

## What good looks like

Three numbered results, each with its score, the sentence saying what carried it and what was never measured, and where the
asset came from; then one line saying which was played, on which bus, at what gain — or the refusal and its reason. With no model
the output is a list of **exactly what is missing and where to get it**, on stderr, with status 2, and nothing is played or
written. The picture is that output.
