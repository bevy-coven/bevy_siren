# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.27 / 4.81 / 5.46**; disk 12 GB free.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.062 | 0.0078 |
| 64 | 0.353 | 0.0055 |
| 256 | 1.135 | 0.0044 |

**The change is not on this path.** `perfcheck` drives `model`'s arithmetic. The reconcile runs inside `start_layers`,
which returns immediately unless `MusicScore` **changed** — so a normal frame does nothing, and the frame that edits a
score does a `retain` over the layer count, a sort of the same, and one query lookup per stopped voice. A score has a
handful of layers; at 256 voices the whole figure is 1.135 µs of a 16.7 ms frame (0.007 %) and none of it is this code.

## What *is* measured

The two probes, before and after:

```
before  remove a layer:   names = ["combat"]   voices still live = 2   (no log line)
        retarget a layer: the score says track.b; the voice is track.a and track.b never played

after   remove a layer:   names = ["calm"]     live layer voices = 1
                          stopped `track.combat`: layer removed
        retarget a layer: the only live cue is track.combat
                          stopped `track.calm`: layer retargeted
        gain edit:        the same entity, the same cue — no restart
```
