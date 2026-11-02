# research.md — iteration 2: a voice that ends must free its slot

## The item

On the crate's own mixer (`--features mixer`), a `Mode::Once` voice never retires. `retire_voices` looks for
`AudioSink::empty` (Bevy's backend) or `Completes::finished` (ours) — and nothing ever writes `Completes`. The
`AudioOutput::finished` method exists in the trait with **zero callers**. So every one-shot the game plays stays a
live `Voice` for the life of the process: the bus count never falls, the table budget never frees, and after
`max_voices` one-shots (32 by default) every subsequent play is refused with `VoiceLimit`. A footstep game is silent
after 32 steps.

That is a bug of the first rank in the loop's ranking: "a message dropped", "a refusal that should not happen", and
worst of all it is silent in the only backend whose whole point is that a bus is one number. Bevy's path does not
have it (`AudioSink::empty` is the end of the sample).

## Sources

1. **Jason Gregory, *Game Engine Architecture*, 3rd ed., §14.5.8.4 "Instance Limiting"** (read in home-still:
   `distill_search "voice stealing priority when too many virtual voices are playing"`, doc
   `Game Engine Architecture, Third Edition … d0213a22…`, lines 23093–23129; ISBN 978-1-138-03545-4 in the edition
   supplied). Reports, verbatim:

   - *"Instance limiting is a means of controlling the number of sounds that are permitted to play simultaneously."*
   - *"a sound engine typically supports only a fixed number of simultaneous voices … so we must use them wisely."*
   - *"Each virtual voice represents a sound that is technically playing, but that can be muted or stopped temporarily
     so it ceases to occupy valuable hardware or software resources."*
   - *"If the listener is beyond this distance … it is considered inaudible and its virtual voice is temporarily muted
     or stopped, **freeing its resources for use by other voices**. The process of automatically silencing virtual
     voices is called *voice stealing*."*
   - *"Sounds might be faded out rather than cut off abruptly when their voice is stolen."*

   **The number this iteration rests on:** a voice is a *resource* with a fixed count, and the resource is returned
   when the sound stops. Gregory's model has no notion of a voice that stops making sound and keeps its slot; that
   voice is the defect.

2. **`rodio` 0.22.2, the vendored source** (`~/.cargo/registry/src/index.crates.io-…/rodio-0.22.2/src/`).
   `Player::empty()` is `self.len() == 0`, where `len` is `sound_count`, decremented when the queued source yields
   `None`; `MixerSource::sum_current_sources` (`mixer.rs:150`) `retain_mut`s a source until `next()` returns `None`.
   So **the backend already knows exactly when a voice ended** — `AudioOutput::finished` is the query, and it is
   already implemented in `src/mixer/mod.rs:158`. The crate simply never asks.

3. **This crate's own rule R4** (`docs/research/siren.md`): *"Per-cue polyphony, per-bus cap and a table-wide budget
   are three hard caps … a request that cannot be made room in is `Refused { VoiceLimit }` with nothing spawned."*
   A budget that is never released turns a hard cap into a permanent lockout, which is the opposite promise.

4. **Bevy 0.20's `bevy_audio`** (`audio_source.rs`, `sinks.rs`): `AudioSource { bytes: Arc<[u8]> }`, and
   `AudioSink::empty()` — the end-of-sample query the crate already uses on that path.

## The technique

A voice's life has three states in the crate, and the mixer path conflates two of them:

| state | mixer path today | what it must be |
|---|---|---|
| waiting for bytes | `Samples`, no `MixerVoice` | unchanged |
| sounding | `MixerVoice(i)` | unchanged |
| ended | `MixerVoice(i)` **forever** | retired: despawned, logged `Stopped { why: "ended" }`, slot freed |

The fix is the query the trait already declares: every frame, `retire_voices` asks the output whether a mixer voice
is finished, and despawns it exactly as it despawns a Bevy voice whose sink is empty. One writer (`retire_voices`)
for the end of a voice, both backends, one sentence in the log.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what a voice is | a fixed resource, returned when the sound stops | Gregory §14.5.8.4 |
| steal order | lower priority first | Gregory §14.5.8.4 |
| the fade a stolen voice gets | "faded out rather than cut off abruptly" | Gregory §14.5.8.4 |
| a stopped sound's slot | "freeing its resources for use by other voices" | Gregory §14.5.8.4 |
| `max_voices` default | 32 | design default (iteration 0) |
| the ended check's cadence | once per frame, on real time | design default: the mix already runs every frame, and a frame at 60 fps is 16 ms of a slot held too long at worst |

No new number is introduced. The change is that an existing number — the budget — is released the way its source
says it is.

## What good sounds like

Nothing audible changes on the frame a voice ends: the sample has already produced its last sample, so the voice was
silent before it was retired. What changes is everything *after* that frame:

- the `SirenMix::voices` count for the bus falls when the sound does, so a panel's "2 of 2" goes to "1 of 2";
- the next play finds room instead of stealing or refusing, so a game's 33rd footstep plays;
- the log gains one `Stopped { why: "ended" }` line per one-shot — the same line Bevy's path already writes.

A reviewer can judge it without hearing it: play 40 one-shots with `max_voices: 32`, and the `Played` count must be
40 with no `VoiceLimit` refusal, the live-voice count must stay at 1, and the log must hold 40 `ended` lines. Today
it is 32 `Played`, then 8 `VoiceLimit` refusals, and 32 voices still live.
