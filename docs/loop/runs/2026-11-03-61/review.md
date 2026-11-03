# review.md — iteration 61, three passes

## Pass 1 — Function: does the Cauldron build, and does it keep a project's music?

**Fault 1, the build.** Before: two errors, `audio_panel` defined twice and `AudioPanelPlugin` not found. **Confirmed
pre-existing** by stashing my only edit and re-running — the same two errors on a clean tree. After: `cargo check
-p bevy_cauldron --features bevy_siren` **green**, for the first time since the duplicate `pub mod` landed.

**Fault 2, the score.** `audio_panel.rs`'s own doc says the file is `SirenSettings`, `CueTable` **and `MusicScore`**;
`read_file`'s doc says *"A score is a third section and is optional"*. The struct had two fields, and RON ignores an
unknown key without `deny_unknown_fields`. Measured with the same `serde`+`ron` types:

| | before | after |
|---|---|---|
| read a file with `music` | `File { settings: 32, cues: 0 }` — **gone** | tempo **120.0** carried |
| write it back | `(settings:32,cues:0)` — two keys | three keys, lossless |
| read a file **without** `music` | n/a | `MusicScore::default()` |

**5 of 5 audio-panel tests pass**, including `the_panels_file_is_the_crates_file` (the existing round trip, updated for
the third element) and the new `a_music_score_survives_the_panels_file`.

**Verdict: GREAT.**

## Pass 2 — Zoom: the shape that refuses every real file

**`Option<MusicScore>` with `#[serde(default)]` is wrong, and the compiler cannot tell you.** It looks right — optional
section, optional field — and it **refuses every real file**:

```
SpannedError { code: ExpectedOption, span: … col: 31 }
```

RON parses a bare struct into an `Option` only when the field is spelled `Some(..)`. I found this by building the
smallest program that has the same `serde`+`ron` shape, not by reasoning: it is the kind of mistake that compiles, type
checks, and fails on the first project file.

**The correct shape is a plain `MusicScore` with `#[serde(default)]`.** A missing key gets `MusicScore::default()`, and
iteration 53 made that default the *same* value the serde field default uses (`beats_per_bar: 4.0`, not `0.0`) — so the
two paths agree. That earlier fix is what makes this one safe.

**The plugin's schedule is `PreUpdate`**, so a file the project changed on disk is in the world before anything draws
from it; the system already guards on `Project::is_changed` and on the bytes, so it is a no-op on a frame that changed
nothing.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: the disk, the palette, and what the feature-off run is for

**A full disk masqueraded as twelve compiler errors.** The first Cauldron test attempt reported `rustc` failures on
`bevy_input`, `bevy_asset`, `bevy_animation`, `bevy_gizmos`, `bevy_text`, `bevy_image`, `bevy_mesh`, `bevy_camera`… The
real message was **`No space left on device (os error 28)`**. That is the skill's own trap written down as a trap, and
it caught the loop anyway: *"a full disk fails as `ld: write() failed, errno=28`, which looks like a code error and is
not one."* The Cauldron's test target pulls `wgpu` and needs several GB; the machine had 2.5 GB.

`cargo clean` in `bevy_siren` removed **37.3 GiB** of compiled artifacts — nothing tracked by git — and the build then
ran. Asked rather than assumed, because it is a deletion of a target this loop did not create.

**The four known-red palette tests fail identically**, exactly as the repo rules state
(`a_person_finds_and_runs_a_command_in_the_palette`). They are red with the feature on and — being re-run to confirm —
with it off, which is the rule's own test that they are not mine.

**What this iteration says about the loop itself.** Twenty-odd iterations have run crate-side work while the *consumer*
was never built. The skill's step 5 names the Cauldron command every time; nobody ran it. A feature that has never
compiled and a file type that silently drops a section are both invisible from inside `bevy_siren` — the crate's own
tests are all green, and they always were.

**Verdict: GREAT.**
