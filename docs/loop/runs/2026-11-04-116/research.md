# Iteration 116 — research: a play whose file failed to load answers `Played`

## The question

A cue names a sound key; `SirenSounds` maps the key to a handle; `AssetServer` loads the file asynchronously. **(1)** What does the crate answer, and what does its log say, when that load *fails* — a typo in a file name, a file nobody delivered? **(2)** What does the dependability literature call a failure the system does not signal, and a report of service that was not delivered? **(3)** What does Bevy's asset server report, and when? **(4)** What does the voice budget do with a voice that never starts?

## Sources

| # | source | what it establishes |
|---|---|---|
| S1 | Avižienis, Laprie, Randell, Landwehr (2004), *Basic concepts and taxonomy of dependable and secure computing*, IEEE TDSC 1(1), DOI 10.1109/TDSC.2004.2 (home-still stem `W2145071552`, p. 14; 5 898 citations) | Service failure modes are classified from four viewpoints, one of them **detectability — "the signaling of service failures to the user(s)"**; a *silent failure* is *"no service at all … delivered at the service interface"*. A failure that is signalled lets the user act; one that is not leaves the user to find it by its consequences — *"may be recognized only after its occurrence, for instance via its consequences"*. `Played` is the crate's service interface to a game's meter and to the panel: **a `Played` for a sound that never sounds is an unsignalled content failure.** |
| S2 | Gregory, *Game Engine Architecture* 3rd ed., §14.5.8.4 (already cited at `mix.rs:837`) | Voices are a fixed resource, and *a sound that stops frees its resources for use by other voices*. A voice that never starts **and never ends** is never freed. |
| S3 | `bevy_asset-0.20.1/src/server/mod.rs:2112-2128` (`LoadState`: `NotLoaded`, `Loading`, `Loaded`, `Failed(Arc<AssetLoadError>)`) and `:1075-1078` (`get_load_state`) | The server **knows** a load failed and says why, with the path. `get_load_state` answers `None` for a Uuid handle (*"Always say we don't have Uuid assets"*), which is what the crate's test handles are. `server/info.rs:283`: a `Failed` asset is only retried by a new request. |
| S4 | `bevy_audio-0.20.1/src/audio_output.rs:109-112` | `play_queued_audio_system` does `let Some(audio_source) = audio_sources.get(&handle) else { continue };` — **it never looks at the load state**, so Bevy waits for ever too. `src/mix.rs:839-863` (`retire_voices`) ends a once-voice only on a sink, a `Completes` or the mixer's `finished`; a voice that never started has none. |
| S5 | `AGENTS.md`: *"Every play answers exactly one of `Played`/`Refused`. A refusal names what is missing; nothing plays on a refusal."*; `docs/loop/needs-decision/undecodable-sound.md` option 4 (*"log the cause, refuse with one variant"* — no public change) | The rule the behaviour breaks, and the precedent for the shape of the fix. |

## The technique

`SirenSounds::failed` already asks the server. This iteration asks it **where the crate decides**: at a play (the picked variant's handle failed → refuse, no voice), and every frame for a voice that was spawned while the load was still in flight (the load failed afterwards → refuse and free it). Both answer with the existing `Refusal::MissingSound { sound }` and put the **asset server's own sentence** — the cause and the path — in the log line, which is option 4 of the existing decision record: no variant is added, so no consumer's exhaustive `match` breaks.

## Parameters the sources report

| `LoadState` of the picked sound's handle | today | after |
|---|---|---|
| asset present in `Assets<AudioSource>` | plays | plays |
| `Loading` / `NotLoaded` | the voice waits (both paths) | the voice waits — **unchanged**; nothing is invented about a slow load |
| `Failed(error)` | `Played`; the voice waits **for ever** | **`Refused { MissingSound { sound } }`**; log: *"the file for sound `x` failed to load: \<the server's error, with the path\>"*; a voice already spawned is despawned and its budget slot freed |
| no record (reserved, Uuid or default handle) | waits | waits — unchanged |

## What good looks like

A developer who types `step_3.wav` for `step_03.wav` reads, at the first play, **one line that names the file the server looked for**, and no `played` line; the voice count stays 0; the cue plays the moment the file exists (a hot reload). The panel's log and `Played` agree with what the speakers did.

## Predictions

Written **after** the discovery probe, which is why the first is a regression statement and not a prediction (`evidence/before/failed-load-probe-on-the-unchanged-tree.txt`: six plays of a typo'd file, six `Played`, no `Refused`, a log of plays and `stolen by ui.click (cue)`). The rest could be wrong.

| # | prediction | how it is checked |
|---|---|---|
| P1 | *(measured)* the unchanged tree answers `Played` for 6 of 6 plays of a typo'd file and `Refused` for 0; the log shows healthy plays and cue steals and **no** sentence about the file | the probe, real `AssetServer` |
| P2 | after: **0 `Played`, 6 `Refused { MissingSound { sound: "typo_click.ogg" } }`, 0 voice entities, 0 counted on the bus**, and every log line names the key and the path | the same probe |
| P3 | a voice spawned while the load is in flight and then fails is **refused in the frame the state becomes `Failed`** (the server processes the failure in `PreUpdate`; the crate's system runs in the same frame's `Update`) | a loader that fails after 150 ms; assert the frame |
| P4 | a load still `Loading` is **not** refused and plays when it arrives; a loaded file is unchanged | a loader that succeeds after 150 ms |
| P5 | cost: the new system with **0 waiting voices ≤ 0.1 µs a frame**; with **256 voices that never started** (headless, no sinks — the worst case) **≤ 10 µs a frame**; the play-time check **≤ 0.5 µs a play** | scratch bench in release |
| P6 | nothing else moves: all 36 `cues.yaml` cases, the 21 rendered WAVs byte for byte, the existing 203 default-feature tests **with no edit to an existing test** (their handles are Uuid, which the server answers `None` for) | the matrix, `cmp` |

## What this does not settle

- **A load that never resolves** (`Loading` for ever, or a handle somebody reserved and never filled) still waits for ever. Nothing says how long a slow disk or a network source may take, and inventing a timeout would be a number without a source.
- **Bytes that load and do not decode, on Bevy's path.** `AudioSource::decoder` calls `.build().unwrap()` (`audio_source.rs:96-101`): the first play of a corrupt file **panics the game**, where the crate's own mixer refuses it (`start_pending`). Out of scope here; recorded.
- **Whether a refusal should skip a bad variant.** A cue of three recordings with one typo is refused every time that variant is picked, as a key with no handle already is; skipping would play two thirds of the plays and hide the third.
- **The layers of a score starting apart** (the other half of *a voice waits for its asset*): measured in this iteration and carved off — `plan.md`.

## Scorecard (written after the measurement; none was wrong, and the finding is what they left out)

**P5 — right, and loose in two places.** The system with nothing waiting costs **0.055 – 0.058 µs a frame** (≤ 0.1 ✓); with **256 running voices 0.059** (a running voice's archetype is skipped, which I had not predicted); with **256 voices that never start, 0.77**
(≤ 10 µs ✓, by an order of magnitude); the check at a play **8.5 ns** for a loaded handle and 137.8 ns for a failed one (≤ 0.5 µs ✓, by 50×). All measured on the first cut; the shipped check returns after one `Assets::get` for a loaded handle, which is less.

**P1 is not a prediction** (the probe came first, as said). **P2 ✓** — 0 `Played`, 6 `Refused`, 0 voices, 0 counted, the log names the key and the path: *"the file for sound `typo_click.ogg` failed to load: Path not found: /tmp/siren-stems/typo_click.ogg"*.
**P3 ✓** — refused in the frame the state became `Failed` (`refused_at == failed_at`, asserted). **P4 ✓** — a load in flight is not refused and plays when it arrives. **P6 ✓** — the 203 default-feature tests ran with no edit (208 now: +4 new, +1 for the reload case), the 21 WAVs are byte-identical, all 36 cases hold.

**What the predictions did not contain, and the build found:** a *failed reload with the bytes still loaded* (`plan.md`, change 1). It was not on the list because the discovery probe used a file that never existed; a prediction table written from one probe has the shape of that probe.

Five predictions, five right as written (P1 was a regression statement, not one). The one thing the build found that the table did not contain is a state I did not think of.
