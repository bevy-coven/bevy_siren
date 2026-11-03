# research.md — iteration 61: the Cauldron's `bevy_siren` feature never compiled

## The item

Rank 4 (Cauldron integration) is the one area this loop has not touched since the panel was written. Checking it found
that **`bevy_cauldron` does not compile with `--features bevy_siren` at all**, and has not since a duplicate `mod`
landed. Two faults, both in `bevy_grimoire`:

```
error[E0428]: the name `audio_panel` is defined multiple times
  --> crates/bevy_cauldron/src/lib.rs:109:1
     |
 53 | pub mod audio_panel;          <- unconditional
     | -------------------- previous definition of the module `audio_panel` here

error[E0425]: cannot find value `AudioPanelPlugin` in module `crate::audio_panel`
     --> crates/bevy_cauldron/src/editor.rs:2428:45
```

**Confirmed pre-existing**: `git stash push` of my only edit, then `cargo check -p bevy_cauldron --features bevy_siren`,
gives the **same two errors**. The feature has been dead for however long the duplicate `pub mod` has been there.

## A second fault: the `music` section is dropped

`audio_panel.rs`'s own module doc says the file is *"`bevy_siren`'s own `SirenSettings`, `CueTable` **and
`MusicScore`**"*, and `read_file`'s doc says *"A score is a third section and is optional"*. The struct it deserialises
has **two** fields, and RON ignores an unknown key on a struct without `deny_unknown_fields`.

**Measured** (a standalone program with the same `serde` + `ron` types):

| | text | result |
|---|---|---|
| read | `(settings: 32, cues: 0, music: (tempo_bpm: 120.0))` | `File { settings: 32, cues: 0 }` — **music gone** |
| write | `File { settings, cues }` | `(settings:32,cues:0)` — two keys |

So a project with adaptive music loses its whole score on any panel read, and the writer has nothing to put back. Not
live today only because `write_file` is called from a test and nothing else — but `read_audio` **does** run, and
`panel.score` did not exist to hold what it dropped.

## A third finding, from writing the fix

The obvious shape — `Option<MusicScore>` with `#[serde(default)]` — **refuses every real file**:

```
SpannedError { code: ExpectedOption, span: … col: 31 }
```

RON parses a bare struct into an `Option` only when the field is spelled `Some(..)`. The correct shape is a **plain
`MusicScore` with `#[serde(default)]`**, which supplies `MusicScore::default()` when the key is absent — and iteration 53
made that default the same score the file loader gives, so the two paths agree.

## Sources

**S1. `audio_panel.rs`'s module doc** — the file is `SirenSettings`, `CueTable` **and `MusicScore`**. The reader's own
sentence, contradicted by its own struct.

**S2. `read_file`'s doc** — *"A score is a third section and is optional: a project with no adaptive music leaves it
out."* "Optional" was implemented as "unread".

**S3. `editor.rs:2428`** — `app.add_plugins(crate::audio_panel::AudioPanelPlugin)` under `#[cfg(feature = "bevy_siren")]`.
The call site is the specification the module failed to meet.

**S4. The crate's own `SirenSettings`/`CueTable`/`MusicScore` derives** use `deny_unknown_fields` on the *crate's* types;
the panel's wrapper struct does not, which is why RON silently ignored the third key rather than refusing the file. A
refusal would have been the louder, better failure.

**S5. Iteration 53's `MusicScore::default()` fix** — `Default` is now `beats_per_bar: 4.0`, the same value the serde
field default uses. That is what makes `#[serde(default)]` on a plain `MusicScore` mean *"the same score a file that
omits the section gets"* rather than two different scores.

## The technique

1. **Delete the duplicate `pub mod audio_panel;`** — line 53 already declares it unconditionally.
2. **Define `AudioPanelPlugin`** — `init_resource::<AudioPanel>()`, `read_audio` in `PreUpdate` (before anything draws
   from it), `on_audition` as an observer. That is what `editor.rs` has been calling.
3. **Carry the score** — a `music: MusicScore` field with `#[serde(default)]` on the reader and the writer, and
   `AudioPanel::score` to hold what was read.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| compile errors with the feature on, before | **2** | measured |
| of those, pre-existing without any local edit | **2** | measured, `git stash` |
| keys a `music`-bearing file parses to, before | **2 of 3** | measured |
| keys written back, before | **2** | measured |
| `Option<MusicScore>` + `#[serde(default)]` on a real file | **`ExpectedOption`** — refuses | measured |
| a plain `MusicScore` + `#[serde(default)]` | parses, default when absent | measured |

## What good sounds like

`cargo check -p bevy_cauldron --features bevy_siren` **green** — for the first time since the duplicate landed — and a
file with a `music` section that reads and writes back with all three sections intact.

## Blocked

`hs distill search` answered `500 … gpu busy` for the eleventh iteration running; this is a consumer-integration fault
measured against the consumer's own source, where no paper bears.
