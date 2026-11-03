# plan.md — iteration 15: the music's mirror reconciles, and a stem is not stealable by its label

## The pick, and why it beats the runner-up

**Pick:** a music layer's voice can die without the layer knowing — a game's `StopCue`, a steal by a same-bus play, or a
refused first start — and `MusicState` keeps a **despawned entity** forever while the score says the layer plays. One of
those paths is rank 1's worst shape: a **stem silently stolen by a UI click**, which the crate's own R4 says must never
happen.

Beats the runner-up (a `Refusal` variant for a stinger on the wrong bus): that one is a decision file and a *message*;
this one is **wrong audio** — the music disappears and nothing says so — plus a documented promise (`unstealable`'s
doc calls a stem unstealable) that the mechanism does not keep.

## The design

Three rules, one module each:

1. **`MusicState` reconciles every frame** (`reconcile_layers`, folded into `start_layers`' existing change branch so no
   new schedule entry). A layer whose `voice` is gone from the world, or is `Stopping`, clears its `voice`. So
   `voice()` never names a despawned entity; `live()` is true; `move_layers` never writes a dead entity's gain.
2. **A layer's voice is unstealable** (`src/play.rs`). `room_for`'s `Live` row learns `is_layer` from the `MusicVoice`
   component the plugin already inserts. `Candidate::unstealable = voice.unstealable || is_layer`. No public type
   changes: `Candidate` is `model`, `Live` is private.
3. **A layer with no voice is asked for again** (`start_layers`). Today `awaiting` is pushed and never cleared, so a
   refused layer is blocked for the process. Key the retry on `voice: None`; drop an `awaiting` entry when a `Refused`
   answers it. A layer the game stopped with `StopCue` is **not** restarted: `StopCue` marks the entity `Stopping`, and
   the reconcile must tell "the budget took it" from "the game asked". The distinguishing fact is the reason string the
   crate already writes: a stopped voice's `Stopping::why` is `"stolen by …"`/`"ended"` (not the game's) or
   `"cue stopped"`/`"stop"` (the game's). Record it on the layer at reconcile time as `restart: bool`.

## Fields

No new field on `CueTable`, `SirenSettings` or `MusicScore`. `LiveLayer` (private) gains `restart: bool`, defaulting
`true`, set `false` when the game stopped the layer.

**Why not a public `MusicLayer::restart`?** It would be a new schema field and a decision file. The default the sources
give is unambiguous for the budget case (restart) and for the game's `StopCue` (do not), and a project that wants the
other is asking for a different behaviour the score can express by re-adding the layer. Carved off and named in
`log.md`.

## Tests

- `a_layer_whose_voice_is_stolen_plays_again` — a one-shot on the same bus cannot steal; assert no `stolen by` line
  for the layer, and 2 live voices.
- `a_layer_whose_voice_ends_is_restarted_at_its_own_gain` — force `StopVoice`... but that is the game's stop: instead
  despawn the entity directly to model a voice that died for a reason the plugin did not ask for. Assert one `Played`,
  the layer's `voice()` is the new entity, and it is back within a frame.
- `a_layer_the_game_stopped_stays_stopped` — `StopCue`; assert no restart after 200 frames, `voice()` is `None`,
  `live()` 0.
- `a_refused_layer_start_is_tried_again` — a bed fills the table; the layer refused; stop the bed; assert the layer
  plays, with no second refusal.

## The carve

- **Not** re-timed to the bar: a stem that restarts still starts from the beginning. Named in `log.md` as item 1.
- **Not** a public `restart` field: a schema change and a decision file.
