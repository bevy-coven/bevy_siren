# Iteration 77 — perf

| Case | Before | After |
|---|---|---|
| 19 — frame cost at 8 / 64 / 256 voices | — | **not re-measured** |

`perfcheck` needs a release build (~2 GB) and the volume was at **100 %, 124 MB free** all iteration. The change touches
no audio path: `semantic::loudness` is behind the non-default `semantic` feature and `ebur128` is an optional
dependency, so the binary `perfcheck` builds is compiled from sources byte-identical to iteration 76's apart from files
that feature does not include.

**Owning it:** this is the second consecutive iteration the perf figure is owed, and the reason is disk, not the work.
The next iteration should run `perfcheck` before anything else.

## The one new cost, measured

`ebur128` is a build-time cost for anyone enabling `semantic`: a clean build of the dependency takes **1.66 s** on an
Apple M4 and produces no C object. Runtime cost is not measured yet; a 3 s mono buffer at 48 kHz is 144 000 frames, and
the analyser is one IIR pass plus a histogram — but that is an inference, not a number, and `#36` (indexing throughput)
owns it.
