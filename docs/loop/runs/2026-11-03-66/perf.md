# perf.md

## The crate's per-frame work

**Not re-measured**, and the reason is exact: no code on the audio path changed. Everything added is behind
`semantic`, off by default, and `cargo tree --edges normal` is unchanged at **413 lines** with no features.

Iteration 64's baseline stands:

| Voices | µs / frame | µs / voice |
|---|---|---|
| 8 | **0.024** | 0.0030 |
| 64 | **0.284** | 0.0044 |
| 256 | **0.667** | 0.0026 |

## The tokeniser's own cost, which is new work

Not timed in isolation, and it does not need to be: it runs **once per query**, on a description of a few dozen
characters, and the whole `semantic_tokenizer` suite — 11 tests, one of which parses the full 3.5 MB vocabulary five
times — finishes in **0.95 s**. The merge search is `O(n²)` in the piece length, which for a word is a handful of
characters.

What it does *not* do: hold the vocabulary per query. [`Vocabulary`] is loaded once and passed by reference.

## A disk figure that belongs in any future number

The gate failed mid-iteration with **`No space left on device (os error 28)`** reported against `proc-macro2` — a
dependency this crate does not control, which is why a full disk reads as somebody else's link error. The disk was at
**121 MB**. `cargo clean` freed **20.8 GiB**.

| | |
|---|---|
| start of iteration | **228 MB free**, `target` 23 GB |
| after `cargo clean` | **22 GB free** |
| when the gate failed | **121 MB free** |
| after the second `cargo clean` | **20 GB free** |

A debug build of this crate plus its dev-dependency costs ~21 GB here. Any iteration that builds more than once will
need a clean between.
