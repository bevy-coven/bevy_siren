# Iteration 110 — plan: the first real run (`#38`)

## Ranked

1. **`#38` a playable example** — chosen. The only open issue buildable in this repository: `#32`–`#34` are `bevy_grimoire`'s,
   `#35` is a person's labelled recordings, `#39` needs a music encoder and a licence audit. Its "blocked by `#33`" is issue-graph
   ordering, not a dependency of the code: the example uses `build_from_model`, `tools::search_music`, `tools::audition` and
   `SirenPlugin`, all built and tested, and the graphs it needs can be made on this machine (the weights are in the Hugging Face
   cache, and torch, transformers and onnx are in the system Python).
2. `#35`, `#32`–`#34`, `#39` — externally blocked.

**Why 1 is more than a docs item:** the semantic layer was built across iterations 56–109 and **had never run on a real model
end to end in this repository**. Every guarantee was a fixture, a synthetic tone or a trace on another machine. The issue asks for
an example; the valuable part is that writing it is the first time anything is wrong in a way no test could say.

## Design

**`tools/export_clap_onnx.py`** — makes `clap-text.onnx`, `clap-audio.onnx`, `tokenizer.json` and `clap-export.txt` from the weights
in a Hugging Face cache, checking each wrapper against the model before writing. The crate does not ship the model, so the
example needs a way to be run by someone who has not already made the graphs.

**`examples/semantic.rs`** (`required-features = ["semantic"]`, as `budget` carries `mixer`) — `<library-dir> "<description>"`:

1. scan the folder; read the index if there is one; **decide what is needed before loading anything**;
2. **pre-flight**: every missing file named, how to make it, exit **2** — before it reads the library, writes an index or opens
   a device;
3. reuse a current index (no audio graph loaded) or build with `indexer::build_from_model`, **refusing to write an index that
   holds nothing**;
4. embed the description, call `tools::search_music` (the call `siren-tools` answers over MCP), print the top `k` with
   `explain()` and provenance;
5. `tools::audition` → a Bevy app with `SirenPlugin` → one `Play` → `Played`/`Refused` → exit when the voice ends.

**`ModelStamp::clap(version)` and `--model-version`** — additive. The index stamp names the export; the first real export on
another machine could not be served by anything but the measured version's string.

## Decided and recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| "no GPU prints what is missing and exits non-zero" | **a missing model is the failure; no GPU is not** | this build has no CUDA path (`Device::available()` is `Cpu`, unconditionally — `model.rs`), so a GPU error would be every run. The first line says the run is CPU fp32; the pre-flight says what is missing and exits 2. Recorded in the guide and on the issue |
| a new dependency | **none** | the example uses only what is in the list; the picture is drawn by Pillow in a script under `docs/media/` |
| the example reuses | `build_from_model`, `tools::{search_music, audition}`, `SirenPlugin` | no second ranking or play path to drift from the tested one |
| the picture | a **transcript still**, not a GIF | the program prints text and ends; the PNG is the verbatim output drawn (`docs/media/transcript_png.py`), so nothing in it is composed |
| the audio graph's path | **`is_longer = false`**, batch 1, no dynamic axes | what the crate sends; found by running it |
| the example's index | **a cache**, rebuilt whole when the folder changes | `incremental::update` takes a closure and has no model-backed form; recorded as the next item |

## Fields

None on `CueTable` / `SirenSettings` / `MusicScore`. New public item: `ModelStamp::clap`. **Defaults and their sources:**
`--k` 3 is `tools::DEFAULT_K` (ADR 0007); `--bus Music` is the bus a music cue plays on; `LONGEST_WAIT_S` 20 is a *design default*
(a longer track is a thing to start in a real player).

## Tests

1. `a_stamp_names_the_export_that_made_the_vectors` (`tests/semantic_index.rs`): `measured_clap()` **is** `clap(MEASURED_CLAP_VERSION)`;
   two exports differ in exactly the version; each is refused by the other **naming both**; the export that made an index loads it.
2. `an_index_from_another_export_is_served_once_its_version_is_given` (`tests/semantic_server.rs`, a **real `siren-tools` process**):
   refused without the flag, naming both versions, nothing on stdout; served with it; a wrong version refused again; the usage
   names the flag and its default.
3. the misuse table gains `--model-version` with no value.

**No test runs the example or the script**, by design: CI has no model. They were run, and the run is the evidence
(`evidence/after/`): the refusal, the first real run before and after, the parity against `transformers`, a played run, and
`siren-tools` driven with the real graph.

## Carved off

- **Model-backed incremental update** — `update_from_model`, the filled seam, so one new file costs one clip, not 5 000.
- **The mel stage** — 26 ms of a 60 ms clip, serial with the tower.
- **GPU figures** (`#36`) — no GPU here.
- A **`list_assets`** tool — `needs-decision/headless-discovery.md` still awaits the owner.
