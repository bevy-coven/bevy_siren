# Iteration 116 — plan: a file that failed to load is refused, with the cause, instead of played in silence

## Ranked

Every open issue was re-read through `issue://` and none changed: `#32`–`#34` are the editor's (built in `bevy_grimoire`, pinned 221 commits back), `#35` waits on a person's anchor pairs, `#39` was decided not to ship (ADR 0005). No `consumer` issue is open; the baseline is green (`baseline-matrix.log`).

1. **A play whose file failed to load answers `Played` and nothing ever sounds** — *chosen*. Wrong output **and** a silent one, on the default cue (`max_instances: 1`, which steals the dead voice at the next play and so *hides* the leak), from the commonest authoring mistake there is (a file name). The crate's own log says *"played"* and *"stolen by ui.click (cue)"* — a healthy-looking sequence for a sound nobody hears. Probed, deterministic, measured (`evidence/before/`).
2. **The layers of a score start apart when their loads straddle a frame** — measured, **carved off**. 8 stems of 3.7 MB through the real plugin, real `AssetServer`, frames paced at 16.667 ms, warm page cache: **3 of 100 starts began in two different frames** (worst spread 1 frame = 16.7 ms, permanent); 0 of 100 for two stems (`evidence/after/stem-skew.rs.txt` is the program). Real, silent, rare on this storage and certain to be commoner on a slow disk, a console or the web. Design below.
3. **Detect an Ogg loop that cannot sound at startup** (iteration 115's #2): rare configuration, a binary asset in `src/`.
4. **`play` copies the whole sample on every mixer play** (`bytes.to_vec()`): performance, one line, below a person's notice on an effect.
5. The hubness correction, the default-floor decision, a third library, a person listening: experimental, the owner's, or not mine.

**Why 1 beats 2:** 1 happens on every load failure with the defaults and breaks a rule the crate states (*every play answers exactly one of `Played`/`Refused`; nothing plays on a refusal*); 2 happens in a correctly authored game a few times in a hundred on fast storage. A refusal is the most valuable bug there is, because it is silent.

## The design

**One predicate**, `sounds::load_error(server, handle) -> Option<String>`: `Some(the server's error text)` exactly when `get_load_state(handle.id())` is `Failed`. `SirenSounds::failed` is rewritten on it, so the question has one home.

**At a play** (`play::answer`): after the picked variant's handle is found, `Failed` → `Err(Refusal::MissingSound { sound })`. `play_requests` takes `Option<Res<AssetServer>>` (its 16th parameter, Bevy's maximum) and, in its `Err` arm, writes the **log sentence** — *"the file for sound `x` failed to load: \<error\>"* — when the refusal is a `MissingSound` whose handle failed; the `Refused` message is unchanged (option 4 of `needs-decision/undecodable-sound.md`: the cause in the log, one variant in the API).

**In flight** (`playback::refuse_failed_loads`, `SirenSystems::Play` after `trim_ogg_loops`): for a voice that has not started on either path (`Without<MixerVoice>`, `Without<AudioSink>`, `Without<SpatialAudioSink>`) **and** whose asset is absent, a `Failed` handle → `Refused { MissingSound }`, the same log sentence, the voice despawned (its budget slot is freed by the next `update_mix`). The mixer's `start_pending` already does exactly this for bytes the decoder refuses; this is the same branch for bytes that never came.

**Not changed:** a load that is still `Loading` waits; a handle the server has no record of waits; no timeout is invented; no `Refusal` variant is added.

## The tests, in the order they are written (each fails first)

`tests/failed_load.rs` (declared in `tests/all.rs`), real `AssetServer`, real fixture file `tests/fixtures/tone.ogg`, **a loader that waits at a gate the test opens** and then succeeds or fails, so the order of events is not a race (first written as a sleep; change 3 below):
1. a play of a sound whose load **has failed** → no `Played`, one `Refused { MissingSound { sound } }`, no voice, nothing counted on the bus, and a log line naming the key, *"failed to load"* and the server's error;
2. a voice spawned **while the load is in flight**, whose load then fails → it is refused **in the frame the state becomes `Failed`**, despawned, and its slot freed; the log says played and then refused;
3. **the control**: a load still in flight is not refused, and plays (waits) when it arrives; a loaded file is untouched;
4. (`mixer`) the same on the crate's own mixer with a recording `AudioOutput`: `play` is never called for a failed load, the voice is refused and gone;
5. **a music layer whose stem failed**: one refusal with the cause, the other layer starts and stays live, and **the log is not written every frame** for the dead one.

## Decisions recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| a new `Refusal` variant (`LoadFailed`)? | **no** | an exhaustive public enum: breaking. `needs-decision/undecodable-sound.md` already weighs this for a *second* cause and recommends option 1 in the next minor; a third cause strengthens it, and the note is added there. Option 4 (the cause in the log) ships now. |
| refuse a bad variant, or skip it? | **refuse when picked**, like a key with no handle | skipping would hide one variant in three. |
| `Played` then `Refused` for a play whose load failed *after* it was spawned? | **yes, both** | `start_pending` already answers an undecodable file that way; the play could not know. A play whose load had *already* failed answers `Refused` only. |
| a timeout for `Loading`? | **none** | a number without a source (`research.md`). |

## Carved off, with its route

**Layers that start together** (rank 2). A barrier in `music::start_missing`: write no `Play` for a batch of layers while any of their handles is `Loading` (`get_load_state`) or reserved and absent (`Handle::Strong` with no record), so every stem's voice is spawned in the frame the last one's bytes are there and both paths start them in one pass; a stem that has `Failed` does not hold the batch (this iteration refuses it). Needs a bound on the wait (a field or a named constant with a source), a way for a panel to say *"holding"* (an additive accessor on `MusicState`), and tests with a reserved handle (deterministic) and a real file (the invariant *no layer starts before every stem loaded*). `docs/guide/music.md` and `music.rs` say *"keeps every layer phase-aligned"*; that sentence is true only when every stem has loaded before the score starts.

## What would change this plan

- If the in-flight refusal cannot be made to fire in the frame the state flips (P3) because the server processes failures later than `PreUpdate`, the test asserts the frame it *does* fire and the doc says so.
- If `get_load_state` is measurably slow with 256 waiting voices (P5), the check moves behind `Assets::get` (already first) and a per-voice marker as in iteration 115's `LoopChecked`.
- If an existing test needs an edit (P6), the premise that Uuid handles answer `None` is wrong and the design is reopened before anything else changes.

## What happened to the plan

It held: probe first, tests first, one predicate, a refusal at the play and a system for the voice already in flight, no variant added. **Two changes the build forced**, both in the first cut and both mine:

1. **The first cut refused a play whose file's *reload* had failed even though the previous bytes were loaded.** A failed hot reload leaves the server's state `Failed` and the old bytes in `Assets` (a failed reload removes nothing), and `answer` read the state alone. Found by asking what `Failed` with bytes *present*
   means, after the five tests were green; a sixth test (`a_file_whose_reload_failed_but_whose_bytes_are_there_still_plays`) went red on the first cut, and `cannot_load` — *failed **and** nothing to play* — replaced the bare predicate at both call sites.
2. **`play_requests` was already at Bevy's sixteen parameters**, so the asset server and the asset table became one parameter. The first form, a derived `SystemParam` struct, put a crate-private type in a public function's signature (`private_interfaces`), which `cargo build` warned about on every command and the matrix's
   `clippy -D warnings` refused. A type alias for the tuple of the two resources is one parameter and adds no type to `play_requests`'s signature. **I ran clippy last, again**; it is now the first command after an edit.
3. **The loader slept 100–150 ms to hold a load in flight, and the premise *"the load is still in flight when the play is asked"* was a race between that delay and the next frame** — false on a machine starved for 150 ms, and in the control test merely vacuous (a file that had loaded would still have been played). Found in review,
   not by a flake. A gate the test opens (`Gate`, `Gated`) replaced the sleep: the loader cannot answer until the test says so, the control now holds the load open for ten frames and asserts nothing is refused and the voice waits, and the music test asserts both layers' voices are waiting before the gate opens.
   23 of 23 runs green, 8 of them with every core held by a busy loop (`evidence/after/gated-loader-stability.txt`); the mutations were run again on the gated tests (`evidence/after/gated-mutation-*.txt`).
