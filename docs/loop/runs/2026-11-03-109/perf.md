# Iteration 109 — perf

Machine: this Mac, **shared and busy** — `uptime` load average **13.3 / 11.3 / 7.5** at the control samples (14.8 at the first,
seconds after a cargo run of mine finished). Every figure here belongs with that number.

## Per-frame audio cost (control)

The host adds no code to the mix, music or play path; `src/` gained `semantic/server.rs`, a `[[bin]]`, and a method on
`semantic::tools`, none of which the per-frame systems call. Four `cargo run --release --example perfcheck` samples, the
crate's per-frame work as an upper bound (every voice costed, including those `update_mix` skips):

| voices | samples (µs/frame) | band across unchanged code (iterations 102–108) |
|---|---|---|
| 8 | 0.063, 0.062, 0.059, 0.060 | 0.041–0.070 |
| 64 | 0.444, 0.359, 0.337, 0.323 | 0.107–0.395 |
| 256 | 1.129, 0.589, 0.543, 0.778 | 0.401–1.339 |

Inside the recorded bands except the **first 64-voice sample, 0.444, above the 0.395 edge** — taken at load 14.8 with my own
build finishing beside it, and the next three read 0.323–0.359. That is the machine's spread, not a result: iteration 102
recorded the same 3× between identical runs. **No speedup or slowdown is claimed.**

## What `siren-tools` costs, which is the number `#28`'s pitfall asks for

`cargo run --release --features semantic --example hostbench` — **new this iteration**, kept apart from `perfcheck` for the
reason `querybench` keeps its halves apart: *a start-up cost and a per-call cost must never be read as the same number.*
Fully analysed libraries (every asset has loudness, spectrum, tempo and key), 512-dimensional vectors.

**Start-up, paid once per process**

| assets | index file | `indexer::read` | `Host::new` |
|---|---|---|---|
| 1 000 | 2.05 MB | 3.4 ms | 0.2 ms |
| 10 000 | 20.51 MB | 34.2 ms | 1.2 ms |

**Per call**, the whole of `Host::handle` — parse, validate, run, serialise — p50 / p99, 60 calls each:

| assets | tool | p50 ms | p99 ms | reply bytes |
|---|---|---|---|---|
| 1 000 | `set_anchor` | 0.015 | 0.022 | 896 |
| 1 000 | `find_compatible_music` | 0.700 | 0.854 | 4 698 |
| 1 000 | `find_compatible_sfx` | 0.742 | 0.750 | 8 244 |
| 1 000 | `explain_match` | 0.018 | 0.022 | 1 894 |
| 1 000 | `search_music` (no encode) | 0.545 | 0.555 | 4 680 |
| 10 000 | `set_anchor` | 0.011 | 0.012 | 896 |
| 10 000 | `find_compatible_music` | **18.210** | 18.873 | 4 704 |
| 10 000 | `find_compatible_sfx` | **18.102** | 18.653 | 8 294 |
| 10 000 | `explain_match` | 0.023 | 0.026 | 1 894 |
| 10 000 | `search_music` (no encode) | 5.255 | 5.527 | 4 680 |

**Whether a long-lived server is the expectation: yes.** 34 ms to read 20.5 MB is paid once; every call after it is a scan of
memory. The stdio transport launches one server per client session, so the process lives as long as the conversation.

**`find_compatible_*` is 3.5× `search_music` at 10 000 assets, and over the 10 ms `rank::PER_QUERY_BUDGET_MS` that
`search_ranked` meets (5.3 ms).** The cause is in the library, not the host: `Toolset::compatible` scores **every** candidate
with the ensemble before truncating to `k`, because the hard rules and the weights reorder the result, so it cannot rank by
cosine first the way `search_ranked` does — and a cosine-first prefilter would change which candidates survive, so it is not a
free speedup. Exact speedups probably exist (four `BTreeMap` lookups per candidate for its features, a `Breakdown` built for
each) and **were not tried**: this iteration is the host, and an agent's call waits on a model for seconds. Recorded here and in
ADR 0008, not changed.

**Not measured: loading the 501 MB ONNX graph, and the text encode.** I did not find the graph on this machine (`/tmp/siren-spike`,
where the iteration-63 probe loaded it, is gone, and a filesystem search for `*.onnx` over 10 MB printed nothing before its
120 s deadline — it did not finish the whole disk). A host started without `--model` pays neither; `search_music`'s figure above is the
scan, the ranking and the serialisation — the crate's part, as `querybench` states for the same reason.

## What a call's wire size says

A `find_compatible_sfx` at `k = 6` is **8.2 KB** and a `find_compatible_music` at `k = 3` is **4.7 KB** — most of it each
result's four-part `breakdown`. That is the price of ADR 0007's *"enough for an agent to choose without another round trip"*,
and it is small next to a model's context: three results are roughly 1.2 K tokens at about four bytes a token — **an estimate,
not a measurement**.
