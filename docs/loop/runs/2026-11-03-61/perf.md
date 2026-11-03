# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0).

**No number is quoted this iteration, and that is the honest result.** `cargo clean` removed 37.3 GiB of the crate's
build artifacts to make room for the Cauldron's test target, so a release rebuild of `perfcheck` would have had to
compile the whole dependency graph again inside this iteration. Nothing in `bevy_siren`'s `src/` changed — the work was
in `bevy_grimoire` and in a serde-gated test — so there is no crate-side frame cost to measure.

**What did change on this machine, recorded because it bounds future numbers:**

| | before | after |
|---|---|---|
| free disk | **2.5 GB** | **31 GB** |
| `bevy_siren/target` | 38 GB (37.3 GiB of artifacts) | cleaned, rebuilt at ~40 s for a `cargo build` |

The next iteration that needs a perf figure has room to produce one.

**A trap that fired, and it is the skill's own.** The first Cauldron test attempt reported compiler errors on a dozen
crates — `bevy_input`, `bevy_asset`, `bevy_animation`, `bevy_gizmos`, `bevy_text`, `bevy_image`, `bevy_mesh`,
`bevy_camera`, `bevy_log`, `bevy_shape`, `bevy_reflect`, `wgpu`. The real message was
**`No space left on device (os error 28)`**. Step 1 of this skill says exactly that: *"a full disk fails as `ld:
write() failed, errno=28`, which looks like a code error and is not one."*

**What `perfcheck` measures** (when it is run): the **crate's per-frame work as an upper bound**: every voice is costed,
including the ones `update_mix` skips because their gain did not move. It is not the audio thread.
