# plan.md — iteration 61: the Cauldron's `bevy_siren` feature never compiled

## The pick, and why it beats the runner-up

**Pick:** `bevy_cauldron` **does not compile** with `--features bevy_siren` — two errors, both pre-existing, confirmed by
`git stash` — and its `read_file` **silently drops a project's entire `music` section**.

Beats every crate-side item on the backlog because all of them are blocked on a public-enum decision, and this one is
the loop's own mandate: *"consumer breakage is fixed in `bevy_grimoire`, as its own commit in that repo."* A feature that
has never compiled cannot have been used, and a score that is read away is a project losing its adaptive music.

## The ranked list

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
2. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
3. A stinger on the wrong bus writes only a log line, not a `Refused` — same decision.
4. **The Cauldron's `bevy_siren` feature does not compile, and the panel drops the `music` section.** Shipped this
   iteration.
5. `needs-decision/exact-pitch-draw.md` — option B for the next minor.

## The design, in `bevy_grimoire` (its own commit, per the repo rules)

1. **Delete the duplicate `pub mod audio_panel;`** at `lib.rs:109` — line 53 already declares it unconditionally.
2. **Define `AudioPanelPlugin`** in `audio_panel.rs`: `init_resource::<AudioPanel>()`, `read_audio` in `PreUpdate` so a
   file changed on disk is in the world before anything draws from it, and `on_audition` as an observer. That is exactly
   what `editor.rs:2428` has been calling.
3. **Carry the score**: `music: MusicScore` with `#[serde(default)]` in both the reader's and the writer's `File`, and
   `AudioPanel::score` to hold it. **Not `Option`** — measured, RON refuses a bare struct into an `Option` (`ExpectedOption`).
   A plain score with `default` gives `MusicScore::default()` when the key is absent, which iteration 53 made the same
   value the serde field default uses.

## A disk note, recorded because it cost two failed builds

The first attempt failed as `rustc` errors on a dozen crates; the real message was **`No space left on device`**
— the skill's own trap, *"a full disk… looks like a code error and is not one."* The Cauldron's test target pulls
`wgpu` and needs several GB; the machine had 2.5 GB. `cargo clean` in `bevy_siren` (37.3 GiB of compiled artifacts,
nothing tracked) freed 32 GB, approved by the owner.

## Tests

- **`crates/bevy_cauldron/tests/audio_panel_reads_the_crates_file.rs::a_music_score_survives_the_panels_file`** — a file
  with `music` reads its tempo and writes it back; a file **without** it gets `MusicScore::default()`.
- **`the_panels_file_is_the_crates_file`** — the existing round trip, updated for the third element.
- **The build itself is the test for fault 1**: `cargo check -p bevy_cauldron --features bevy_siren` was red with two
  errors and is now green.
