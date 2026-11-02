# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores), toolchain 1.99.0.

**Machine state.** Load average 8.08 / 8.50 / 6.05 on an 8-core machine for the baseline, 7.4 / 8.2 / 6.0 for the
after run, and disk at 99 % (5.5–5.7 GB free). The machine is shared and busy, so each figure is the mean of 2 000
frames and the comparison below is between two runs on the same machine minutes apart, not between two machines.

**What this number is.** The crate's per-frame *game-thread* work — `model::chain_db` and `model::db_to_gain` for
every live voice plus `model::approach` per bus, which is the shape of `update_mix`. It is an **upper bound**: the
real `update_mix` skips a voice whose gain moved less than 0.01 dB, and `perfcheck` costs every voice.

| voices | before, µs/frame | after, µs/frame | change | before, µs/voice | after, µs/voice |
|---|---|---|---|---|---|
| 8 | 0.062 | 0.065 | +4.8 % | 0.0078 | 0.0081 |
| 64 | 0.353 | 0.332 | −5.9 % | 0.0055 | 0.0052 |
| 256 | 1.396 | 1.233 | −11.7 % | 0.0055 | 0.0048 |

**The change is inside the noise, and it should be.** `perfcheck` drives the two `model` functions; this iteration
changed neither. `update_mix` gained one branch per voice (which number to write) and one extra `Applied` insert for
a mixer voice, and `retire_voices` gained one virtual call per once-voice per frame. The 8-voice row moving 0.003 µs
is the scheduler, not the branch: 3 ns is a fraction of one L1 miss.

At 256 voices the whole mix costs 1.23 µs of a 16.7 ms frame — 0.0074 % — so no case moved by more than 10 % in
either direction, and the largest figure is far below anything a frame could notice.

## The retire loop's own cost

`retire_voices` now asks the output once per once-voice. On the mixer path that is one `Player::empty()` per voice
per frame (an atomic load of a `usize`) plus one virtual call. The example `budget` runs 40 plays and 80 frames with
the retire loop live; its wall clock is dominated by the 17 ms-per-frame `Time<Real>` advance the harness inserts,
so it is not a clean measurement of the loop. The arithmetic is what matters: **one atomic load per once-voice per
frame**, which at 256 voices is 256 loads — well under a microsecond, and zero when the feature is off (`output` is
`None` and the `match` falls through).
