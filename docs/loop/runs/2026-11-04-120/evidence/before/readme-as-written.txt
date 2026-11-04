Probe: the README's "Quick start" block pasted VERBATIM (evidence/before/readme-quick-start.rs.txt) as an example of this crate, on the UNCHANGED tree (HEAD 0a0e359), this Mac (Apple M4 Pro, a real window,
a real output device). Files a reader following the keys would have: assets/audio/step_01.ogg and assets/audio/step_02.ogg (made for this iteration, see assets/README.md).

=== 1. cargo build --example scratch_readme (what the reader's compiler says)
warning: function `walk` is never used
  --> examples/scratch_readme.rs:18:4
warning: function `hear` is never used
  --> examples/scratch_readme.rs:22:4
(the snippet defines `walk` and `hear`, and its `main` adds neither to the app)

=== 2. BEVY_ASSET_ROOT=$PWD ./target/debug/examples/scratch_readme, left running 7 s, everything the console showed
2026-11-04T09:28:44.448167Z  INFO bevy_diagnostic::system_information_diagnostics_plugin::internal: SystemInfo { os: "macOS 26.6.2 Tahoe", kernel: "25.6.0", cpu: "Apple M4 Pro", core_count: "14", memory: "48.0 GiB" }
2026-11-04T09:28:44.498430Z  INFO bevy_render::renderer: AdapterInfo { name: "Apple M4 Pro", ... backend: Metal ... }
2026-11-04T09:28:44.690345Z  INFO bevy_pbr::cluster: GPU clustering is supported on this device.
2026-11-04T09:28:44.690412Z  INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported on this device.
2026-11-04T09:28:44.746709Z  INFO bevy_winit::system: Creating new window scratch_readme (67v0)
2026-11-04T09:28:44.829482Z ERROR bevy_asset::server: Path not found: /Users/rebekahbrittain/bevy_siren/assets/audio/step_02
2026-11-04T09:28:44.829483Z ERROR bevy_asset::server: Path not found: /Users/rebekahbrittain/bevy_siren/assets/audio/step_01
(nothing else: no sound, and not one line from this crate)

What this shows
 1. NOTHING PLAYS, for two independent reasons. (a) `main` never adds `walk`, so no `Play` is ever written. (b) The keys are "step_01" and "step_02", and `load_table` asks the asset server for `<dir>/<key>` ("audio/step_01"),
    which is not a file: the files are step_01.ogg and step_02.ogg. The only sign is Bevy's own `ERROR bevy_asset::server: Path not found: .../assets/audio/step_01`, which does not name the crate, a cue or a sound key.
    With the extension in the key and `walk` scheduled, 118's probe played five of five (runs/2026-11-04-118/evidence/before/front-door-probe-on-the-unchanged-tree.txt).
 2. The README's `hear` is the third defect (evidence/before/hear-probe.rs.txt, run on the same tree, headless, one play):

      one play; the log holds 2 entries:
        0.30s played `footstep.grass` (s) on Effects at 0.00 dB      <- the crate's own entry, at the time it happened
        0.00s played `footstep.grass` (s) on Effects at 0.00 dB      <- `hear`'s copy, `at_s: 0.0` written by hand

    `hear` is documented as showing "which recording the player heard, and why it was chosen". It copies what the crate already wrote into the crate's own log, with a time it made up, and prints nothing.
 3. The rest of the front door holds: the three commands under "Try it" were run as written (the budget one in a debug build, not `--release`):
      cargo run --example music    -> t = 0.01 s .. 4.80 s .. 7.20 s, "music: transitions on the bar: yes"
      SIREN_FRAMES=90 SIREN_SHOT=... cargo run --example mixer -> "siren: frames 90 mean_ms 16.829", a 15 KB still, exit 0
      cargo run --features mixer --example budget -- /tmp/siren-budget-120 -> 40 asked, 40 played, 0 refused, 4 files written
    So the defect is the Quick start block and what the guide says about keys (docs/guide/cues.md:147-149: "`step_03` sends you to `audio/step_03.wav`" - the crate asks for `audio/step_03`).
