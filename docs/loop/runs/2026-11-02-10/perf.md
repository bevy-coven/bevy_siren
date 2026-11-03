# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.73 / 5.71 / 6.26**; disk 15 GB free after clearing my own spent build directories
(it had fallen to 674 MB and `ld` failed with `errno=28`, the loop's documented trap).

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.065 | 0.0082 |
| 64 | 0.354 | 0.0055 |
| 256 | 1.389 | 0.0054 |

**What is added to `update_mix`.** Two `BTreeMap::retain` calls over maps whose length is the bus count (four by
default, at most what a project lists) and one `Vec<BusLevel>` clone **only when the table's list is empty**, which is
the default configuration — a four-element clone and four allocations per frame, once per frame, not per voice. A frame
at 256 voices costs 1.389 µs of 16.7 ms (0.008 %); `perfcheck` drives `model`'s arithmetic only, so it cannot see either
change and the run is the machine (load 4.7 against iteration 9's 4.1).

If the default-table clone ever mattered, hoisting the resolved list into a `Local<Vec<BusLevel>>` keyed on the
settings' revision would remove it; measured cost says it does not, and an added `Local` is state that can go stale.

## What *is* measured

The probes, before and after:

```
before  mix.db after a 4→2 bus swap:  [Ambience, Dialogue, Effects, Music]
        bus_db("Ambience")           = -3.3 dB   for a bus not in the table
        deepest_duck()               = Some(("Ambience", -12.0))   forever

        buses: [], a cue on Effects  -> played, mixed at -80.0 dB, no refusal

after   mix.db after a 4→2 bus swap:  [Effects, Music]
        bus_db("Ambience")           = -80.0 dB
        deepest_duck()               = None

        buses: [], a cue on Effects  -> played, mixed at 0.0 dB on the crate's four
```
