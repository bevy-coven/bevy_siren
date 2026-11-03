# research.md — iteration 17: a voice nobody plays, and a budget slot nobody gets back

## The item

`Backend` is a **public** resource whose flag a host may set (`pub struct Backend(pub bool)`), and `spawn_voice`
decided *"the mixer owns playback"* from that flag alone:

```rust
let entity = spawn_voice(&mut commands, request, &started, now, backend.0);
```

With `Backend(true)` and **no `Output`**, the voice carries no `AudioPlayer` (the mixer owns it) and has no mixer (none
exists). It cannot sound, so nothing ends it, so `retire_voices` never retires it: **it holds its budget slot for the
life of the process while `Played` says it played.** Measured:

```
Backend(false), Bevy's path:   voices 1, AudioPlayer 1
Backend(true),  NO Output:     voices 1, AudioPlayer 0   <- silent, permanent
```

`start_pending`'s doc already promises the opposite: *"without an `Output` — the default build, or one whose device
failed to open — this system returns at once and Bevy's `AudioPlayer` owns every voice."* The promise was about a
resource the spawn never read.

## Sources

**S1. Gregory, *Game Engine Architecture* 3e, §14.5.8.4** (vendored: `docs/research/siren.md` R4). A voice is a **fixed
resource**, and *"a stopped voice frees its resources for use by other voices"*. A voice that can never sound and can
never stop is the worst case of that rule: it consumes a slot forever. R4's whole budget argument — per-cue polyphony,
per-bus cap, table budget, steal by priority then age — assumes voices are recoverable. A leaked slot turns the budget
into a permanent lockout, which is what cases 20 and 21 of `cues.yaml` exist to catch.

**S2. Bevy 0.20's audio path**, read in the vendored registry (`bevy_audio-0.20.0/src/audio.rs`). A voice sounds because an
entity carries `AudioPlayer`; Bevy starts it, waits for the asset, and ends it. There is no other way for the crate to
make a sound on Bevy's path, so "no `AudioPlayer`" is exactly "not played".

**S3. `MixerBackend`'s module doc** (`src/mixer/mod.rs`, *Failing like Bevy fails*): when `open()` returns `None`,
*"the plugin then inserts no `Output` … so voices fall back to Bevy's `AudioPlayer`."* The documented behaviour — the
fallback — is the behaviour this iteration makes true.

**S4. `models::SILENT_DB` / the crate's rule R6, and `docs/research/siren.md`'s "fail loud"**: *a bad cue is refused
with a reason, never played wrong.* A voice that is answered `Played` and then never sounded is the one failure mode
that is **neither** a sound nor a refusal: the game believes it played and nothing can bring it back.

## The technique

Decide ownership from the fact, not the flag:

```rust
let mixer_owns = backend.0 && output.is_some();
```

One boolean, read where the voice is spawned, from the two resources whose agreement is the claim. This is the same
shape `start_pending` already uses (`let (Some(mut output), Some(assets)) = … else { return }`): the system asks whether
there is a mixer, not whether a flag says there should be.

**Why not make `Backend` a getter that computes it?** `Backend` is a `Resource` with a public tuple field; changing it
to a method is a **public API change** and would break `Backend(true)` in this crate's own examples and in any host. The
fix belongs where the false decision was made, and it needs no API change at all.

**Why not refuse the play instead?** Because the fallback exists and is already documented and already works: with no
`Output` the crate's own mixer is not there, and Bevy's `AudioPlayer` is a perfectly good path. A refusal would turn a
recoverable configuration into a lost sound.

**What this does not fix.** A host that sets `Backend(true)` and inserts an `Output` that is a *fake* still has a
consistent world; a host that sets `Backend(false)` while the mixer feature opened a device gets Bevy's path for every
voice, which is also consistent (the mixer sits idle). Both are the host's choice and both play.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a voice is a fixed resource; a stopped voice frees its slot | — | S1, §14.5.8.4 |
| an entity with no `AudioPlayer` makes no sound on Bevy's path | — | S2 |
| with no `Output`, Bevy's `AudioPlayer` owns every voice | — | S3 (the promise this iteration keeps) |
| a play answered `Played` must sound or be refused | — | R6 / S4 |

## What good sounds like

A world with `Backend(true)` and no `Output` **plays its sounds** — on Bevy's path, which is what the docs said would
happen — and its voices end and free their slots. Nothing about the audio changes; the difference is that a slot is not
lost. The test that would have caught it asks one question a reader can check without code: *does the voice have a
player?*
