# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **A `MusicLayers` change naming a layer the score does not have was silently dropped.** `move_layers` applied each target
  with `if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name)`, so a name that matched nothing vanished:
  no log line, no refusal, no counter — the one message in the crate with no failure path. A typo in a layer name was
  therefore a change that did nothing and said nothing. Measured: `MusicLayers::new(vec![("nope", -6.0)])` left the log
  holding only the layers' own `played` lines. The lookup is now a `let … else` that writes
  ``refused `nope`: this score has no layer `nope`, so the change names nothing`` and carries on, so **one bad target does
  not cost the others** — a message naming a real layer and a typo moves the real one and refuses the typo.
  `SIREN_LAYER=typo` captures it.


- **A stinger at `duck_db: 0.0` logged a duck it did not make.** `arm_stinger_ducks` armed `DuckClock::manual` and pushed
  `Entry::Ducked { by_db }` unconditionally, while `model::active_duck_db` returns `0.0` for any `by_db >= 0.0` — so a zero
  depth (or a positive one, meant as a lift) wrote a line saying the music stepped back and the music did not move.
  Measured: `Stinger { duck_db: 0.0, .. }` produced `[Ducked { by_db: 0.0 }]` with `Music` unmoved. The duck is now armed
  and logged only when it can lower the bus, using the same `by_db < 0.0` test the envelope itself uses. A stinger's own
  hit still plays; only the dip is conditional. Iteration 52 fixed the same inert path for a table rule, where a report
  could reach it — a stinger is a message, so the log was the only place a person could see it and it said the opposite.


- **`SirenSounds` never released a sound key, and its handles kept whole audio files resident.** The map exposed
  `insert`/`get`/`len`/`keys`/`load_table`/`missing`/`failed` and **no remover**, and `load_table` **merges** — it asks for
  the keys it has no handle for and never drops one the table has lost. Measured: two tables of ten different keys loaded
  in turn left **twenty** keys held. The map overhead is small (~62 KiB per thousand) but a `Handle<AudioSource>` is what
  keeps an asset alive, and `AudioSource` is `bytes: Arc<[u8]>` — **the whole encoded file** — so every stale key was an
  audio file held for the life of the process. `SirenSounds::retain_keys(&table)` now drops the keys the table no longer
  names; call it before `load_table` on a reload. Not a wrong-output bug: a stale key is never read, checked at every
  `sounds.get`.

### Added

- **`SirenSounds::retain_keys`** — drop the keys a table no longer names. Additive; `load_table`'s contract is unchanged.


- **A cue the table no longer had kept its `VariantPicker` for the life of the process.** `SirenChoices::pickers` is keyed
  by cue name and an entry is created on a cue's first play; nothing removed one. `reset_histories` clears each picker's
  *contents* and is not a retain, so it did not help. Measured: 200 cue names played, the table cut to ten, and **200
  pickers still held** — each with an owned copy of its recent sound keys. `SirenChoices::retain_cues(&table)` now drops
  the pickers of cues the table has lost, and `play_requests` calls it **only on a frame where the table changed**.
  Variant streams are byte-identical to before: a cue the table keeps keeps its history.

### Added

- **`SirenChoices::retain_cues` and `SirenChoices::picker_count`** — the retain, and the number it holds. Additive.


- **A duck rule the mix table dropped kept its start for the life of the process.** `DuckClock::started` is keyed by
  `(source, target)` and was only ever emptied by `clear()`, which nothing calls; `prune(now_s)` retained the *manual*
  envelope list and not the rules'. Measured: **2000 `fire`s followed by a `prune` at t=1000 still held 2000 entries**,
  and 100 table edits left 100 — reachable in the Cauldron, which rewrites and re-reads `family/bevy_siren.ron` on every
  edit. `DuckClock::retain_rules(&settings)` now drops a start no current rule names, and `update_mix` calls it every
  frame beside `prune`. A running duck is unchanged across a retain (−8.0 dB at the same moment before and after), and
  the call costs **13 ns per frame**, measured at 1, 10 and 1000 held starts.

### Added

- **`DuckClock::retain_rules` and `DuckClock::rule_starts`** — the retain, and the number it holds, so a panel can draw
  it and a test can assert it. Additive; `prune`'s signature is unchanged.


- **`MusicScore::default()` was not the score the file loader gives.** `beats_per_bar` carried
  `serde(default = "MusicScore::default_beats_per_bar")` — **4.0** — while `derive(Default)` gave it **0.0**, so the same
  score was four beats and a `Some(2.0)` bar from RON and **zero beats with `bar_s() == None`** from `Default`: no beat
  grid at all, every `NextBar` and `NextBeat` change landing on the frame it was asked for, and
  `problems()` reporting the crate's own default as *"beats_per_bar is not a positive number"*. `Default` is now written
  by hand and returns `default_beats_per_bar()`, so the two spellings of the field are one number.
  `tests/music.rs::a_default_score_quantises_to_the_bar` drives a `NextBar` change from a `Default` score and asserts it
  lands at 2.0 s; it fails on the old tree.
- **A duck at `by_db: 0.0` lowered nothing and said nothing.** `active_duck_db` returns `0.0` for any `by_db >= 0.0`
  and the mix keeps a reduction only `if db < deepest` with `deepest` starting at `0.0`, so such a rule fired, reached
  the log, held for its whole `hold_s` and left the bus exactly where it was — while `SirenSettings::problems()` answered
  `[]`. A `by_db` above zero was already reported as a raise, and a non-finite one cannot be written in RON at all
  (`by_db: nan` answers `Expected float`), so zero was the one depth both authorable and silent. The report now says so,
  naming `by_db` rather than `hold_s`, because for a zero depth with a real envelope the depth is the wrong field. The
  rule's behaviour is unchanged: it still fires and is still logged. `SIREN_MIX=zero` captures it.


- **A cue's `AvoidRepeat` window was frozen at its first play.** `SirenChoices::pick` builds a cue's `VariantPicker` on
  its first play and reused it for the app's life, while `pick_from` took `avoid` fresh on every call — and `remember`
  bounded the window by the deque's **allocator** capacity, not by a number anyone chose. Measured: four keys at
  `avoid: 3` on a picker built at `capacity 1` produced the **byte-identical** 24-pick sequence to `avoid: 1` —
  `adbcbcabcacabcabcbabcbcb` where `avoid: 3` requires `adbcadbc…`. Sixteen of twenty picks returned within three of
  themselves. `VariantPicker` now keeps an explicit `window`, **grows to meet the `avoid` passed to `pick_from` and
  `pick`**, and `Default` remembers one pick instead of none. A table an editor reloads with a larger `avoid` takes
  effect on the next play. Reachable in the Cauldron, whose audio panel rewrites and re-reads `family/bevy_siren.ron`;
  `reset_histories` had no caller in the crate, its tests, or the panel, so nothing else cleared it.

### Added

- **`VariantPicker::set_capacity` and `VariantPicker::capacity`** — grow or shrink the remembered window, and read it.
  Additive, so no consumer breaks.
- **`SIREN_MIX=zero`** in `examples/mixer` — a table whose only duck is `by_db: 0.0` with a real hold and fade, so the
  panel shows the zero-depth sentence as its whole `Problems` block. `SIREN_MIX=broken` already fills the block's
  three-sentence cap, and a fault added there would be the invisible `and 1 more`.
- **`examples/render` writes each cue's variant stream** as a `variants <cue>:` line — the rule, the sound count, the
  twelve picks and the longest run. Variant selection is audible and was otherwise only inferable from a wav.

- **`examples/render`'s `mix.wav` was written past full scale.** It applied `sum_db` of **every** cue's gain — `+18.58 dB`
  — to one 0.5-amplitude tone, so the file was `4.2×` full scale: `ebur128` read **`+0.2 dBFS` true peak, `−4.3 LUFS`**.
  `sum_db` of ten cues is the number if all of them sounded at once through nothing; the file's job is to let the mix be
  heard, so it is now written at `settings.master_db` and reads **`−6.1 dBFS`, `−18.6 LUFS`** (below the −14 a game mix
  targets). The `mix:` line states what the sum is and what the file is. Example only.
- **`examples/mixer`'s own table no longer clips.** `Effects` reached `+6.0 dBFS` and `Music` `+4.0 dBFS` with two
  voices, which the panel has shown since iteration 49 and which the guide's new headroom section makes actionable:
  `Effects` `0.0 → −7.0` and `Music` `−2.0 → −7.1`, both a decibel inside the rail (`2 × 10^(−7/20) = −0.98 dBFS`). The
  panel's peak clause disappears because nothing reaches 0 dBFS. Example only.

### Added

- **The guide states that `master_db` is the headroom trim, and that it is inside `worst_peak_db`.** The decision file
  `docs/loop/needs-decision/above-unity-gain.md` recommends *no* report for loudness and names the master as *"the tool
  for it"*; nothing had tested that sentence or written it where a reader looks. `tests/mix.rs::
  the_master_is_inside_the_worst_peak` measures it — one voice at unity reads `0`, `−6`, `−20` dBFS at masters of `0`,
  `−6`, `−20` — and `docs/guide/buses.md` now says so, with why `problems()` stays deliberately silent about loudness.


- **`SirenMix::worst_peak_db(bus)` — the worst-case peak a bus can reach this frame, in dBFS.** The crate could mix
  past full scale and had no way for a game to ask: `max_voices` is a **sample cost** (*"fewer than a mixer's worth of
  sample cost"*, `docs/research/siren.md`), never a headroom guarantee, and nothing combined a bus's gain with its live
  voice count — though `model::sum_db` already held the arithmetic R6 states (*"several sounds **sum in linear**"*).
  Eight sources at 0 dB sum to **+18.06 dB**; two on a −6 dB bus reach **+0.02 dBFS**. It is an **upper bound** — the
  crate mixes samples and never reads their amplitude — and reads `SILENT_DB` for a bus with no voices. Additive: a new
  method, no field or variant. The shipped example's own table reaches **+6.0 dBFS** on `Effects` and **+4.0 dBFS** on
  `Music` with two voices, which the panel now shows.

### Fixed

- **`examples/render` no longer calls clipping "the budget's job".** Its line read *"307 samples would clip (the budget's
  job, not the gain's)"* — `voices-8.wav` has **307 samples at the rail**, measured from the WAV, and the budget is a
  sample cost that never claimed to bound the sum. It now says so and names `SirenMix::worst_peak_db`.


- **A bus asking for more voices than the table allows is reported.** `SirenSettings::bus_voice_limit` is
  `min(cap, total)`, so `BusLevel::new("Effects").max_voices(16)` under `max_voices: 4` mixes **four** — and neither
  `BusLevel::max_voices`'s doc (which names only the `0` sentinel) nor the guide's *"a bus's own `max_voices` caps it
  further"* said that a **larger** number does nothing. `problems()` gains a twelfth sentence —
  *"bus `Effects` asks for 16 voices but the table's max_voices is 4, so it mixes 4"* — additive, since the report is
  `Vec<String>`. A bus at `0` (the sentinel) and a cap **equal** to the table's stay silent; both are asserted. The
  behaviour is unchanged: the bus still mixes the table's four.


- **A duck that starts on the bus it lowers is reported.** `Duck::new("Effects", "Effects")` is accepted and
  **quietens its own voices**: `fire_all` matches rules on `source`, `duck_db` matches them on `target`, and for a
  self-duck one bus is both — so a footstep pulls down every other effect for the rule's whole envelope, and itself with
  them. Measured: `-6.0 dB` on the frame its own voice starts, and the bus mixed that much lower. `problems()` gains an
  eleventh sentence — *"the duck on `Effects` starts on the bus it lowers, so every voice on it quietens itself"* — which
  states the mechanism rather than forbidding it, because a self-sidechain is a real device. Additive: the report is
  `Vec<String>`. The behaviour is unchanged.


- **A duck with neither a hold nor a fade is reported.** `Duck::new(source, target)` leaves `hold_s: 0.0` and
  `fade_s: 0.0`, and `active_duck_db` returns `0.0` past the hold when `fade <= 0.0` — so the shortest rule a project
  can write **fires, is recorded in the log, and is inaudible on the very frame it fires**. Measured:
  `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time` reads `0.0` from the bare rule and `-8.0` from the
  same rule with `.hold_s(0.25).fade_s(0.6)`. The crate's own `default_ducks()` chains both, which is why its pair
  works and why nothing said otherwise. `SirenSettings::problems` gains a tenth sentence —
  *"the duck on `Music` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s"* — the
  report is `Vec<String>`, so this is additive. The **behaviour is unchanged**: a project that relies on an inert rule
  today still gets one, it is simply named now. A raising rule is not reported twice (the guard is `by_db < 0.0`).


- **The panel's `Problems` block draws the mix table's faults, not only the cue table's.** It read
  `table.problems_in(&settings)` alone, so a mix table with `max_voices: 0`, a NaN `master_db` or a bus listed twice
  drew `Problems: none` **while every bus read −80 dB** — the report said a table it could not run was fine. It now
  concatenates `SirenSettings::problems()` with the cue table's, and `SIREN_MIX=broken` produces the former so the block
  can be seen doing it. Example only; no `src/` behaviour changed.

- **The panel's cue list is bounded at eight rows.** `SIREN_FAULTS=many`'s fourteen cues drew twelve rows and pushed
  the `Problems` block and then the whole log off a 1280×720 capture — measured, the log's heading was not on screen and
  the lowest ink sat at `y = 719` of 719. It now says `and N more; the log below is what played`, the same rule the
  block under it already followed, and the log's line count subtracts that row as well as the block's. Margins after:
  9 px clean, 10 px broken, 10 px with fourteen cues.

- **`SirenSettings::problems`'s doc lists all nine checks it performs.** It named six — `master_db` not a number, a
  bus's gain not a number, and a duck's hold or fade were unlisted — while `MusicScore::problems`'s doc has listed all
  nine since iteration 34. Four of the nine sentences had **no test at all**; `tests/mix.rs::problems_names_every_mistake`
  now walks all nine, each shown to be its own fault, and each of the four reddens it when removed. No behaviour changed.

### Added

- **`MusicState::blocked(layer)` reports a virtual voice.** A score may name more layers than the budget has room for;
  when the layer's `Play` is refused the plugin records the live count and asks again when it falls below. That state
  was private, so a panel drawing only `layer_db` printed `-80.0 dB` for a layer **waiting for a voice** and `-80.0 dB`
  for one authored silent — the same string for two different answers to *"why is the music quiet"*. The accessor
  returns `Some(n)` (the count it waits for) or `None`, and is additive: no variant, field or exhaustive `match`
  changes. Gregory §14.5.8 — *"a virtual voice is a sound the game believes is playing but that has no hardware
  voice"*. Proved by `tests/music.rs::a_refused_layer_start_is_tried_again_when_room_frees`, whose mutation (returning
  `None`) reddens it.

### Fixed

- **A sound whose owner is despawned now says it stopped.** `Play::on(entity)` parents the voice with `ChildOf`, so
  despawning that entity — the ordinary end of a sound on a vehicle or a torch — takes the voice with it. Neither
  backend logged it: the crate's own mixer freed the slot in `start_pending` since iteration 7 and wrote **no line**,
  and on Bevy's path the voice entity is simply gone, so `retire_voices` — which runs during `Mix`, one frame *before*
  Bevy applies the despawn — never sees it. The log's last word about such a sound stayed `played` while the mix counted
  one fewer voice, against `"ended"`, `"stop"`, `"cue stopped"` and `"stolen by …"` for every other end.
  `mix::log_despawned_voices` records each live voice's cue and whether it had a `ChildOf`, reads
  `RemovedComponents<Voice>`, and writes one `stopped \`<cue>\`: owner despawned` for a parented voice that vanished —
  on **both** backends, once. A game that despawns a voice it never parented still logs nothing. No public type,
  variant or field changed. Proved by `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` (no feature flags,
  so the default backend) and `tests/mixer.rs::a_despawned_voices_mixer_slot_is_freed` (which also pins that the line is written once).

### Added

- **`examples/mixer` shows a music layer waiting for a voice.** The `Music` block appends `waiting` to a layer's gain
  when the budget could not give it a voice — `combat -80.0 dB waiting` — and `SIREN_TIGHT=1` gives the `Music` bus one
  voice so the state occurs in a capture (`docs/media/mixer-waiting.png`). The cap is on the bus, not the table, so the
  other buses keep their room and the picture isolates the point.
- **`examples/mixer` shows a parented sound ending.** A `parent` step spawns an owner, plays `engine.idle` on it with
  `Play::on`, and despawns the owner two frames later — so the panel's log now carries
  `0.68s stopped \`engine.idle\`: owner despawned` beside `Effects 1 / 2 voices`. The script is now
  `steal,spatial,parent,layer,duck,stinger,poly,refuse`.

### Changed

- **A steal says which cap took the voice.** `Entry::Stopped`'s reason was `stolen by footstep.grass` for **three
  different budgets** — the cue's `max_instances`, its bus's `max_voices`, and the table's `max_voices` — so an author
  who saw it had no way to know which number to raise, and a step that took three voices logged two identical lines.
  It now ends `(cue)`, `(bus)` or `(table)`: `stolen by sword.hit (bus)`. `room_for` returns which cap chose each victim.
  Every existing `starts_with("stolen by")` check still matches. Proved by
  `tests/mix.rs::a_steal_names_the_cap_that_took_the_voice` (forces the bus cap) and a `(cue)` assertion in
  `a_cue_respects_its_own_polyphony`; the mutation — one format string without the cap — reddens both.

### Added

- **`examples/mixer`'s cue row shows how many of a cue may sound.** The list drew name, bus, mode, gain and `priority` —
  and `priority` decides who *survives* a full budget, while `max_instances` decides whether the next play may start at
  all. `footstep.grass` admits two (`x2`), `ui.click` one (`x1`), and the documented `max_instances: 0` reads `all` and
  still pairs with `UnlimitedInstances` in `Problems` rather than inventing a number. A new `poly` step asks a cue three
  times in one frame, so the row's `x2` is proved by two live voices and one `(cue)` steal — and it runs last in the
  default script, so the still's log keeps the stinger, the dialogue duck and the spatial play above it.

- **A played gain is written to two decimals.** `Entry::line` was
  ``"{at_s:.2}s played `{cue}` ({sound}) on {bus} at {gain_db:.1} dB"``. One decimal cannot show a distance law: the
  default `Attenuation::Inverse { near_m: 4, rolloff: 1 }` gives **−6.0206 dB** at 8 m — `cues.yaml` case 14's number —
  and `-6.0` is also what a cue authored `gain_db: -6.0` produces, so the log could not tell a distance from a fader and
  disagreed with the panel's own cue row, which prints two. `gain_db`'s *field* keeps full precision; only the sentence
  rounds. **A host that parsed the log's one-decimal gain gets two decimals.** A duck's `by_db` stays at one decimal: it
  is an authored depth, not a computed product. Proved by
  `tests/model.rs::a_played_gain_is_written_to_two_decimals`, whose mutation (`{:.1}`) reddens it.

### Added

- **`examples/mixer` shows a placed sound.** The panel had **no spatial cue and no `.at(..)`** — grep found zero of
  either — so the crate's spatial path (three attenuation models, a panner, and `Refusal::SpatiallyMute`) was tested and
  never drawn. The table gains `bang` (`spatial: true`, `Effects`, priority 30), a `spatial` step plays it at **8 m**,
  and its cue row carries the law and the gain that distance gives — `inverse at 8 m -6.02 dB` — computed with
  `model::distance_gain`, the same function a play uses. The log's other numbers are unchanged. The log also shows **six**
  lines rather than seven: the eighth cue row costs 21 px, and with seven the lowest ink sat at `y = 719` of 719 — no
  margin, a clipped glyph.

### Fixed

- **The audio panel names the duck whose depth it shows.** A bus row printed `SirenMix::ducked`'s depth — the **minimum**
  over every active rule — beside the **newest** `Ducked` entry's `by`. With one duck those are the same rule; a stinger
  firing over a line of dialogue puts two on one bus, and the row read `duck -8.0 dB by stinger` when −8.0 belonged to
  `dialogue.greeting`. Two numbers from different sources, presented as one fact. The row now names the newest entry with
  **its own** `by_db` when the two differ, and the plain `by <cue>` when one rule explains the number — including a rule
  that is recovering, which is not a second cause. Example only; no `src/` file changed.

### Added

- **The panel plays the stinger.** `music.stinger` had been in `examples/mixer`'s table since it was written and **no
  step ever wrote a `Stinger`**, so the crate's second music feature and the one duck whose cause is a mechanism
  (`by: "stinger"`) had never appeared on screen. `SIREN_KEYS=…,stinger,…` (in the default script before `refuse`) fires
  it, and `docs/media/mixer.png` is now captured on that frame. No `src/` file changed.

### Changed

- **`max_instances` is documented as a budget on plays, not a standing count.** The field read *"how many of this cue
  may sound at once"*; the crate keeps it at the moment of each play, so a table edited while voices are sounding leaves
  them playing and refuses the next request rather than cutting a voice the game did not stop. `Cue::max_instances`,
  `CueTable::insert` and the buses guide now say which reading the crate takes, and two tests hold it — including the
  mutation that would adopt the other one. No behaviour changed.
- **`UNSTEALABLE_PRIORITY`'s rule is now stated directly in the suite.** It was proved through
  `the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue — a path that takes a
  paragraph to explain. Two tests now ask it the way the rule is written: a priority-0 request against a priority-200
  voice is refused with nothing stolen, and a stealable voice still plays beside one so neither can pass vacuously. No
  `src/` file changed; the mutation check (`steal` ignoring `unstealable`) reddens both new tests **and** the old one.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- **`CueTable::problems_in(&SirenSettings)`** — the report against **the buses the mix actually has**. `problems()` is a
  wrapper over `problems_with(&DEFAULT_BUSES)`, so a project that lists its own buses — fully supported, and the answer
  `answer`'s `UnknownBus` check, `update_mix` and `SirenSettings::problems()` all read — had every cue on a custom bus
  reported as `UnknownBus` while the crate accepted and mixed it. `problems_in` reads `settings.bus_names()`, the same
  answer every other reader uses. `problems()` keeps its documented default-four meaning and now says what it assumes.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`SirenSettings::problems()` agrees with the mix about what a bus is, and names each fault once.** The report built
  its own bus set from `self.buses` instead of asking `bus_names()`, the one answer every other reader uses — so a table
  with `buses: []`, which `docs/guide/buses.md` documents as *"the crate's four … everywhere"*, was reported as having
  **four missing buses** while the crate mixed it correctly. And a duck fault was pushed once per rule, so the crate's
  own default pair (`Dialogue → Music`, `Dialogue → Effects`) printed the same sentence twice — a panel reading
  `problems()` verbatim read one fault as two. The report now asks `bus_names()`, and deduplicates by **sentence**: two
  rules that are the same fault are one, while two different faults about one bus (a duck starting on it, a duck lowering
  it) stay two. No public type changed.
- **A music layer's gain that is not a number no longer silences the stem behind a `NaN` fader.** Iteration 18 gave the
  crate one rule for a non-finite caller value — the neutral value for what it measures — and applied it to the numbers
  that enter `answer`. A layer's `gain_db` entered by another door: `NaN` went into `layer_db`, `target_db` and the
  voice's `base_db`, `chain_db` silenced the sink (correctly), and **`MusicScore::problems` said nothing**. The layer is
  now normalised with the same `model::finite_or` in `start_layers` and in every `MusicLayers` target, so a layer with a
  bad gain starts at `0.0` dB and the fader agrees with the sound; `SILENT_DB` is finite and is untouched, so a score
  that starts a layer silent still does. `problems()` gains one sentence, `"music layer \`calm\` has a gain that is not
  a number"` — the score's report is `Vec<String>`, so no public enum changed.
- **A gain or detune that is not a number is the neutral value, not silence — and `Played` reports the rate the device
  is at.** `chain_db` returns `SILENT_DB` for a non-finite part, which is right for a bus level the crate computed and
  wrong for a number an author typed: a cue with `gain_db: NaN` (which a RON file can contain) **played silent** while
  `Played` reported `-80.00` as though that level were authored, and nothing said why — which R6 forbids (*"a play is
  never silent: `Played` or `Refused`, both with the reason"*). `semitones_to_ratio` already played a non-finite detune
  at `1.0x`, but `Played.pitch` carried the raw `NaN`. One rule now covers every non-finite caller value, matching what
  `distance_gain` already does for a distance (`docs/guide/spatial.md`): no gain, no detune, and `Played.gain_db` /
  `Played.pitch` report the numbers the voice is **actually at** — a detune past the clamp reports `±48` semitones,
  which is the rate that plays. `chain_db`'s silencing rule is untouched, so a silent bus still silences its voices. The
  stricter answers (a `CueProblem::BadGain`, a `Refusal` variant) are recorded as
  `docs/loop/needs-decision/bad-gain-problem.md`: both public enums are exhaustive and a new variant breaks every
  consumer's `match`.
- **`Backend(true)` with no `Output` no longer gives a voice nobody plays.** `Backend` is a public resource whose flag a
  host may set, and `spawn_voice` decided *"the mixer owns playback"* from that flag alone: with the flag set and no
  `Output`, a voice carried **no `AudioPlayer`** (the mixer owns it) and had **no mixer** (none exists), so it could not
  sound — and a voice that cannot sound never ends, so it was never retired and held its budget slot for the life of the
  process while `Played` said it had played. Ownership is now `backend.0 && output.is_some()`, which is what
  `start_pending`'s doc already promised (*"without an `Output` … Bevy's `AudioPlayer` owns every voice"*). A world with
  a mixer is unchanged: its voices still carry no `AudioPlayer`, so the two paths cannot both play one request.
- **A music layer naming a cue that ends no longer machine-guns the play path.** `MusicScore::problems` states the rule
  — *"names cue `hit`, which is not a loop, so it cannot hold a layer"* — and the plugin ignored it: a `Mode::Once` cue
  was started, its voice was retired (a once voice **ends**; `retire_voices` only skips loops), and the layer was asked
  for again **every frame** — 10 plays in 10 frames, with no `Refused` and no log line, while an unknown cue was refused
  with a reason. `start_layers` now reads the cue the layer names this frame, does not start a cue that is not a lock
  `Mode::Loop`, keeps the layer in `names()`, and writes **one** line in `problems()`' own words. A table edit that
  makes the cue a loop starts the layer on the next frame. A `Refusal` variant for the same fact is recorded as
  `docs/loop/needs-decision/not-a-loop-refusal.md` — it is public API and waits for the owner.
- **A music layer's voice could die and the layer never notice.** `MusicState` wrote the entity a layer's `Played`
  reported and never reconciled it, so a voice stopped by the game (`StopCue`), taken by the budget, or refused at its
  first play left `voice(name)` naming a **despawned** entity, `live()` counting it, and the stem silent for the rest of
  the process. The mirror now reconciles every frame: a dead voice is cleared, a stem that died behind the plugin's back
  is restarted at its own gain, a `StopCue` stays stopped, and a refused first start is retried **when a voice frees**
  (one refusal line, not one per frame).
- **A music layer is unstealable whatever priority its cue authored.** `Voice::unstealable`'s doc says *"music, stingers
  and dialogue are not stolen"*, but the mechanism was `priority >= UNSTEALABLE_PRIORITY`: a layer authored with the
  crate's default `priority: 0` was called music and treated as a footstep, so a `ui.click` on the same bus at
  `max_voices: 1` stopped the stem. The budget now reads the `MusicVoice` the plugin already inserts; the newcomer is
  refused instead.
- **A `MusicScore` edited at runtime now reconciles its voices, not only its layer list.** `MusicScore` is a resource a
  project and the Cauldron edit, and `start_layers` only ever *added* layers: a layer **removed** from the score kept a
  looping voice playing forever, ownerless (`retire_voices` skips loops by design), and a layer **retargeted** to a new
  `cue` kept the old sample while the new one never started. Both disagreed with the score in silence. A removed or
  retargeted layer's voice is now marked `Stopping` with the true reason (`"layer removed"` / `"layer retargeted"`), so
  it fades over `DEFAULT_FADE_S` and the log says why; a retargeted layer starts its new cue the same frame. A **gain**
  edit still keeps the voice — that is what `move_layers` moves. `MusicState::names()` keeps its documented score order.
- **A `StopVoice` for a voice that has already ended no longer panics.** `stop_requests` took the entity straight from
  the message and issued a command on it; `retire_voices` despawns a one-shot the frame it ends, and `Played::entity` is
  documented as the handle a game keeps for `StopVoice` — so holding it one frame too long hit Bevy 0.20's default error
  handler, which **panics** (`Entity despawned … its index now has generation 1`). The request is now resolved against
  the live voice query first: there is nothing left to stop, so nothing happens and nothing is logged. The same guard
  makes a second `StopVoice` for a voice already fading a no-op, so a fade is begun once.
- **An empty `buses` list mixed every voice at −80 dB.** `answer`, the editor's panel and `SirenSettings::bus_names()`
  all treat a table listing no buses as the crate's four, but `update_mix` iterated `settings.buses` directly — so with
  `buses: []` a cue on `Effects` was **accepted and then mixed at −80 dB**, with no refusal and no log line. The mix now
  iterates the same resolved list `bus_names()` returns.
- **`SirenMix` no longer keeps a bus that left the table.** `db` and `ducked` were written by insertion and never pruned,
  so a project that swapped `SirenSettings` kept every bus from the previous table at its last level: `bus_db` answered
  for a bus that did not exist, `deepest_duck()` named one forever, and a removed bus's duck was frozen at its deepest.
  Both maps are now pruned to this frame's buses before the bus loop, which leaves the previous value readable for a bus
  that stayed — so a fader still eases rather than steps.
- **A stinger that cannot play no longer ducks the music.** `play_stingers` wrote the `Play` and armed its duck in the
  same breath, but `Play` is answered later in the frame — so a refused stinger left the `Music` bus down for a sound
  nobody heard, and `SirenLog` stated the duck **and** the refusal on adjacent lines. A new `arm_stinger_ducks` arms
  the duck when `Played` arrives, after `Play` and before `Mix`, so the dip still lands on the frame the stinger fires.
  Stinger refusals now write a `Refused` message as well as a log line, and a cue **not in the table** is refused as
  `UnknownCue` instead of inheriting `Music` through `unwrap_or`. A wrong-bus stinger has no `Refusal` variant that is
  true, so its sentence stays in the log: `docs/loop/needs-decision/stinger-refusal.md`.
- **A non-spatial cue given a position is no longer routed through the two-ear panner.** `spawn_voice` decided Bevy's
  `PlaybackSettings.spatial` — and whether to attach a `Transform` — from `request.position.is_some()` alone, so a cue
  authored `spatial: false` and played with `.at(..)` went through `rodio`'s panner, which applies `min(1, 1/d²)` as
  well as a pan: **−38.7 dB at 8 m**, while `Played.gain_db` and the log reported **0.00 dB**. One predicate now
  decides the attenuation, the panner and the `Transform` (`cue.spatial && position.is_some()`). **A project that
  relied on the accident loses a pan it was never promised, and its meter was wrong while it happened.** Refusing such
  a play instead is `docs/loop/needs-decision/non-spatial-position.md`.
- **The crate's own mixer reuses a voice slot instead of retaining every sound ever played.** `MixerBackend` pushed a
  `VoiceTrack` — a `rodio::Player` and its queues — per play and never removed one, so a game on the `mixer` backend
  held one player for every sound it had ever played: 400 one-shots held **400 slots** on a real device. `play` now
  reuses the first slot whose voice has ended, bounding the list by the peak *concurrent* count (6 for those 400), and
  the index it returns packs a slot **and a generation** (`model::pack_voice`), so a `MixerVoice` held by an entity the
  world has not finished despawning addresses nothing rather than the slot's next occupant. `MixerBackend::voices()`
  now reports slots, not sounds played. No public type or trait signature changed.
- **A sample the decoder refuses names the sound key, not the cue.** `start_pending` reported a decode failure as
  `MissingSound { sound: voice.cue }`, putting the cue's name (`footstep.grass`) in a field documented as the key a
  project maps to a file (`step_03`) — so a corrupt `step_03.wav` sent a person looking for a file that never existed.
  It now recovers the key from the same cue and variant `answer` chose. The variant's sentence covers both of its
  causes, *"no handle or undecodable bytes for sound `step_03`"*; a variant that distinguishes them is
  `docs/loop/needs-decision/undecodable-sound.md`. No public type or field changed.
- **A `VoiceLimit` refusal names the bus only when the bus refused.** `room_for` has three hard caps — the cue's
  `max_instances`, its bus's `max_voices`, the table's `max_voices` — and returned one uninformative `None` from all
  of them, so every refusal reported the cue's bus. A probe proved the table case blamed **`Ambience`** for a cue
  playing on `Effects`. It now returns the cap (private `Cap::{Cue, Bus, Table}`): `bus: Some(name)` only for the
  bus's own cap, `bus: None` otherwise, which is what the field's own doc says (*"the bus that had no room, when one
  did"*). **A host that matched `VoiceLimit { bus: Some(_) }` to mean "the limit refused at all" now sees it only for
  the bus case**; `VoiceLimit { .. }` still matches every one. A payload that also names the cap is
  `docs/loop/needs-decision/refusal-names-its-cap.md`.
- **`MusicLayers::fade_s` is the crossfade it says it is.** The field was accepted, stored in `Pending`, and read by
  nothing: `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms and
  nothing said so. `model::tau_for_fade` derives the time constant from `approach`'s own definition
  (`fade_s / ln(1000)`, an arrival within one part in a thousand), and each layer carries the fade it was last given.
  `fade_s(0)` keeps `MIX_TAU_S`, so the builder's default is unchanged.
- **A frame with no elapsed time holds a layer's gain.** `approach` returns its target for `dt <= 0`, which is right
  for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend the whole
  authored fade in one step. `move_layers` now holds the gain when `dt <= 0`.
- **`SirenSettings::seed` is now the stream the crate draws from.** `SirenChoices` was `init_resource`d from a crate
  constant and never rebuilt, so a project's `seed:` was read by nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. `play::seed_choices` replaces the stream (and every variant history) when the mix
  table's seed changes, so two seeds are two streams and one seed replays itself — `cues.yaml` case 17, which could
  not pass before. No field moved: the setting existed and now works.
- **A stinger's `duck_s` is a hold, not a fade.** `play_stingers` passed it as `DuckClock::manual`'s *fade* argument
  with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired: a −9 dB stinger read
  **−5.01 dB** at its midpoint and the music swelled back underneath it. It now holds for `duck_s` and releases over
  `DEFAULT_FADE_S` (0.12 s), reading **−8.99 dB** at the same point. A project-chosen release is
  `docs/loop/needs-decision/stinger-release.md`.
- **A voice that ended now frees its slot on the crate's own mixer.** With the `mixer` feature, a `Mode::Once`
  voice was never retired — the crate asked Bevy's `AudioSink`, which that backend does not create, and nothing ever
  set `Completes` — so it kept its place in the voice budget forever. After `max_voices` (32 by default) one-shots,
  every later play either stole an audible voice (with a full budget, silence was hidden behind a steal) or, for an
  unstealable cue, was refused `VoiceLimit` for the life of the process. `retire_voices` now asks
  `AudioOutput::finished`, the query the backend already implemented, and writes the same `ended` log line Bevy's
  path writes. Gregory §14.5.8.4: a stopped voice *"frees its resources for use by other voices"*.
- **A voice's own gain is written on both backends.** `update_mix` skipped a `MixerVoice` entirely, so a music layer
  moved by `MusicLayers` stayed at the gain it started at, and a stop fade began from silence and despawned on its
  first frame — a cut where the crate promises a fade. The mix now writes the voice's own gain into the mixer and
  its whole chain into a Bevy sink, records which in `Applied`, and one writer still owns both.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- `examples/budget` (`--features mixer`) — a headless run of N one-shots against a voice cap, with a scripted output
  that ends each sample, writing the live count and the refusals per frame as a CSV, a text summary and an SVG plot.
  It is the evidence harness for "an ended voice frees its slot", from `docs/loop/cues.yaml` cases 20 and 21.
- `tests/mixer.rs` — three tests behind the `mixer` feature: a one-shot's slot is freed (fails on the unfixed tree), a
  loop is not retired when its source ends, and a fading voice is not also reported ended.
- `examples/render` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  WAVs and a `render.txt`, so a change can be judged by spectrogram and loudness without an ear. No public API
  changes: it drives `bevy_siren::model` directly.
- `examples/perfcheck` — µs per frame and per voice at 8, 64 and 256 voices.
- `.omp/skills/build-loop` — the loop itself, as a project skill: one iteration from orient to log, with the four
  traps that cost an iteration each written down where the next run reads them.
- `docs/loop/cues.yaml` — the fixed cases every iteration runs, each with its observation and the source for its
  number; `docs/loop/runs/` and `docs/loop/needs-decision/` for a run's evidence and the questions the loop will not
  decide itself. `docs/loop-log.md` becomes `docs/loop/log.md`.
- `docs/loop/needs-decision/remove-completes.md` — whether the public `Completes` component, which cannot ever be
  true, is removed in the next minor version.

## [0.1.0] - 2026-11-02

The first release: sound for Bevy 0.20 as data, mixed in buses.

### Added

- **`examples/mixer`'s panel shows the music score.** It shipped a two-layer score and drew none of `MusicState`'s seven
  panel-facing accessors, so `track.calm` and `track.combat` appeared as ordinary cue rows and the `Layer` step's real
  crossfade on the bar was invisible. A `Music` block now lists the layers with their live gain and, while a layer moves,
  `-> target` — `calm -4.0 dB   combat -80.0 dB` at rest, `calm -79.9 -> -80.0   combat -3.1 -> -3.0` mid-crossfade. The cue
  list no longer repeats a cue the score owns, and the log shows **seven** lines instead of eight: the block costs 68 px
  against the 63 px the three cue rows freed, measured from the captures, and seven keeps an 11 px margin where eight
  leaves 1 px and a clipped glyph.

### Fixed

- **`examples/mixer` shows a refusal.** Its claim is *"the log of what played and why"*, and the scripted run never refused
  anything — `steal` steals, `duck` ducks, `layer` moves, every step plays — so the `Log` block had never drawn a `Refused`
  line, the one message the skill calls the most valuable. A `refuse` step now asks for a cue the table does not have
  (`Refusal::UnknownCue`) and joins the default script, so `cargo run --example mixer` with no environment shows
  `refused \`no.such.cue\`: no cue of that name` at the top of its log. The log also shows **four** lines instead of seven
  when the `Problems` block has content: measured, the fault table costs three log lines (two cue rows, two problem
  sentences) and seven leaves a 1 px margin, a clipped glyph. The GIF is captured at native 1280 rather than down-scaled.

- **`examples/mixer`'s header counts what the panel shows.** It read `9 cues` from `table.cues.len()` while the cue list
  showed **seven** — iteration 36 moved the score's two layers into a `Music` block and the header was not part of that
  change, so the one number a reader can check by counting rows was wrong. It now names the split:
  `7 cues, 2 layers, 4 buses, 4 voices`, with the cue count `saturating_sub` the layers so a layer naming a cue the table
  lacks — which `problems()` reports — cannot underflow. Under `SIREN_FAULTS=1` it reads `9 cues, 2 layers` over nine rows,
  which a hard-coded count could not do.

- **`examples/mixer`'s GIF samples a fade by the second, not by the update.** `SIREN_SHOT_DIR` kept every
  `SIREN_SHOT_EVERY`th **update**, but a layer fade is `MIX_TAU_S` = 50 ms — about six updates at the example's ~8.3 ms —
  and the shipped GIF sampled every twelve: **0.5 samples per fade**, so the `Music` block's `-> target` arrow appeared by
  luck of the phase. `SIREN_SHOT_EVERY` is now **hundredths of a second** (12 = 120 ms) and the capture keeps a frame when
  that much real time has passed. At `EVERY=4` (40 ms) the capture kept 47 frames and **11 consecutive** carried the arrow
  (`calm -76.2 -> -80.0   combat -6.9 -> -3.0`), where the old sampling gave half a frame.

- **`MusicScore::problems`'s doc lists all nine checks it performs, and `MusicLayer::cue` says `must`, not `should`.**
  The report's doc named four checks while the function performed nine — the two added since (a gain that is not a number,
  and a layer on a bus other than `Music`) were never added to it, so a reader trusted a list and stopped. The layer's doc
  still said the cue *"should"* name the `Music` bus after the plugin began **refusing** one that does not. Both now describe
  the mechanism, and `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine so the list and the code
  cannot drift apart again.

- **A music layer naming a cue on another bus no longer plays there.** `MusicLayer::cue`'s doc says the cue *"should name
  the `Music` bus"*, and nothing checked: a layer naming a loop on `Effects` started on that bus, so `MusicState` reported
  two layers live while one was mixed on the effects bus, `MusicLayers` could not move it, and `MusicScore::problems` said
  nothing. The plugin now refuses such a layer — one line per layer, and the layer is not started — and `problems()` gains
  the same sentence, word for word, so the report and the log cannot disagree. This is the rule `play_stingers` already
  applies to a stinger on the wrong bus.

- **A queued `NextBar` change follows the score's tempo.** `When::NextBar` is documented as *"on the next bar, quantised
  with `next_boundary`"*, and `move_layers` computed that instant once, at request time, from the tempo then — `Pending`
  stored only `at_s`. A `MusicScore` is a resource the Cauldron edits, so a tempo change while a change waits left it
  scheduled against a bar that no longer existed: queued at 160 bpm for 1.5 s and fired at 1.44 s after the score said
  **80**, where a bar is 3.0 s — the crossfade began **mid-bar**; after a change to **240** it fired a whole bar late. The
  queue now remembers which boundary it was quantised to and the tempo it used, and re-quantises on a frame whose tempo
  differs, never moving a change earlier than that frame. A queued change at an unchanged tempo does not move at all, and
  a `When::Now` change still applies on its own frame.

- **The panel's log is no longer clipped.** `examples/mixer` asked `SirenLog::recent(10)` in a fixed column that fits about
  8.5 lines, so the last entry was cut off **mid-glyph** in every capture, including the README's — measured, the lowest
  ink reached `y = 719` of 720. `SirenLog::CAPACITY`'s own doc says *"a panel shows eight"*, and the panel now shows eight:
  the lowest ink sits at `y = 708`, with 12 px of margin and no sliced glyph. Example only.

- **A bus row says which cue ducked it.** `examples/mixer`'s bus row read `duck -8.0 dB` and never said *by what*, so a
  person had to find the matching log line to learn which cue lowered the bus — while `Entry::Ducked` records it and
  `SirenMix::deepest_duck`'s own doc says it exists *"for a panel that says what a line of dialogue did"*. The row now reads
  `duck -8.0 dB by dialogue.greeting` while the duck holds, and drops the clause the frame it releases. No public field
  added: `SirenMix` knows *how far*, `SirenLog` knows *by what*, and the panel already holds both. Example only.

- **The panel's `Problems` block no longer hides the log.** It was unbounded, in a fixed column with no scroll, and sat
  above the log — so a table with six faults pushed the `Log` heading and its ten lines off the bottom of a 1280×720
  capture: the panel lost the section that answers *"why is the music quiet"*, exactly when there was most to read. The
  block now shows three sentences and, when there are more, `and N more; the log below is what played`. Two faults (the
  README's GIF) and a clean table are unchanged. `SIREN_FAULTS=many` reproduces the case. Example only.

- **The README shows the panel the code draws.** Both images it carries — `docs/media/mixer.png` and `docs/media/mixer.gif` —
  were generated *before* the panel gained its `Problems` block, so a developer's first look at the crate showed a panel
  that no longer existed; the alt text named *"eight cues … eight log lines"* where the shipped table has nine cues and
  the log shows ten. They are regenerated (the still from the clean table, the GIF with `SIREN_FAULTS=1` so the `Problems`
  block and the `<- problem` markers are visible), the alt text is read off the new pictures, and the orphan
  `docs/media/mixer-panel.gif` that iteration 26 created and no README named is gone.

- **A voice whose entity is despawned frees its mixer slot.** `Play::on(entity)` parents the voice to a game entity, and
  despawning that entity is the ordinary way a parented sound ends. On Bevy's path the sink is a component on the voice's
  own entity and dies with it; on the crate's own mixer the voice's number was also a component there, so the despawn
  destroyed the number and **`stop_voice` was never called** — the `rodio` player kept sounding with its slot held for the
  life of the process, and the two backends answered differently from the same input. `start_pending` now keeps the starts
  it handed out in a `Local` and, once per frame, stops any whose entity is no longer in the world. No public type changed;
  Bevy's path is untouched.

- **A duck ends its hold when the voice that justified it stops.** `Duck`'s own doc says a duck holds for `hold_s`
  *"because a voice starting on `source`"* is playing; `DuckClock` ran the full hold whatever happened to the source, so a
  line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering — **0.85 s of duck
  for a sound that had already stopped**. `DuckClock::source_went_quiet` now rewinds a rule's clock *once* when its source
  bus has no live voice, so the fade is already underway and keeps advancing. A loop that keeps playing holds for its
  full `hold_s`, a second line extends it, and a one-shot that ends by itself keeps its hold — measured, one row of four
  moves. No public type changed: `duck_db` keeps its signature.

- **`examples/mixer`'s panel shows the crate's own verdict on its cue table.** It drew each cue's name, bus, mode, gain
  and priority and **not** `problems()`, so a cue on a bus the mix did not have, or one whose `pitch.low` was past
  `pitch.high`, looked exactly like a healthy one — the picture said a broken cue was fine. The panel now has a
  `Problems` block (listing `table.problems_in(&settings)`, or `none`) and marks a bad cue's row with `<- problem`, so the
  column says *which* cue and the block says *what*. `SIREN_FAULTS=1` adds two deliberate mistakes to the capture so the
  view can be proved; the shipped table stays clean.

- **`examples/mixer` no longer panics on the command its own doc gives.** Its capture knobs are documented as optional
  beside three that default, but `SIREN_KEYS` had none: unset gave an empty step list, and `stage_keys` evaluated
  `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release --example mixer` died with *"attempt to calculate
  the remainder with a divisor of zero"* **and wrote no still**. The default is now the documented `steal,layer,duck`
  script, and a `SIREN_KEYS` that parses to nothing prints one line naming the typo before using it — a capture that
  silently staged something else would put the wrong picture in an evidence folder. The explicit paths are unchanged.

- **A `Refused` entry outlives a burst of `Played` ones.** `SirenLog` evicted the oldest entry whatever it was, so one
  refusal was pushed out by exactly `CAPACITY` (256) plays — at a 32-voice scene writing 640 entries a second, about
  **0.4 s**, which is the silent-failure class the log exists to prevent: the line explaining why nothing played,
  discarded by lines saying something did. When the buffer is full, `push` now drops the **oldest routine** entry
  (`Played`, `Stopped` or `Ducked`) first, because those happen at frame rate while a healthy game produces no refusals
  at all; if every entry is a refusal the oldest still goes. The buffer stays 256 entries — same memory, same
  `recent`/`iter`/`len` — and only *which* entry leaves changes.

- **`SirenChoices::seed()` reports the seed the stream was built from, not the one asked for.** `Rng::new(0)` maps a zero
  state to `ZERO_SEED` (`0x5EED_51AE_0000_0001`), which is also `SirenSettings::default_seed()` — so `seed: 0` and the
  default are **one stream** — but `SirenChoices` recorded the argument, and `seed()` — documented as *"the seed this
  stream was built from"* — named a number the stream did not come from (`0` for a stream that started at `0x5EED…`).
  `Rng` now remembers what it was built from (`Rng::seed()`, `Rng::ZERO_SEED`) and `SirenChoices::seed()` returns it, so a
  log line, a panel and a test agree with the sound. No sound changes; no public type changed.

- **`AvoidRepeat` now avoids the sound, not the slot index.** A cue may list one file twice — a copy-pasted line, a
  generated table — and two slots naming one key are two indices and **one sound**. `VariantPicker` refused indices, so
  under the *default* `Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording three
  times running (`… b a a a b`) — the machine-gun footstep `Repeat` exists to kill — and `["a","a","a","b"]` played four
  in a row. `VariantPicker::pick_from(&[&str], avoid, rng)` builds the avoided window from **keys**, so after a sound is
  heard every slot naming it is refused; `pick(len, ..)` is unchanged for every caller that has no keys, and the two
  agree whenever keys are distinct. A cue with one sound still repeats — the crate cannot invent a recording.

### Added

- **A cue table as data.** `CueTable` of `Cue`s, each with variants, a bus, a gain, a pitch range, a repeat rule, a
  polyphony limit, a priority and an attenuation. Every play answers exactly one `Played` or `Refused`, and a refusal
  names what is missing (an unknown cue, no variants, a missing sound key, an unknown bus, a full budget, no listener).
- **Buses that duck.** `SirenSettings` with `Music`, `Effects`, `Ambience` and `Dialogue` out of the box, per-bus level,
  mute and voice cap, and `Duck` rules that lower one bus under another for a hold and a fade. `SirenMix` is this
  frame's mix — one number per bus, ducks and master included — for a panel or a game's own meter.
- **A voice budget that refuses politely.** Per-cue, per-bus and table-wide caps, with stealing by lowest priority then
  oldest, an `UNSTEALABLE_PRIORITY` above which nothing is taken, and a `Refused { VoiceLimit }` — nothing spawned —
  when the budget cannot be made room in.
- **Layered music on the bar.** `MusicScore` of `MusicLayer`s and a tempo; `MusicLayers` moves their gains with
  `When::{Now, NextBeat, NextBar}`, quantised by `model::next_boundary` from tempo alone. A layer at `SILENT_DB` stays
  in the mix so every layer is phase-aligned; a `Stinger` plays at once and ducks the music.
- **Spatial attenuation.** `Play::at(position)` with `Attenuation::{None, Inverse, Linear}`, Bevy's `SpatialListener` as
  the ears, and the backend panner's own distance law divided back out so a cue's curve is the one a project hears.
- **A log of what played and why.** `SirenLog` keeps the last 256 decisions — played, refused, stopped, ducked — each
  with the real time it happened.
- **Two backends.** Bevy's `AudioPlayer` by default, or the crate's own summing graph on `rodio` behind the `mixer`
  feature, where a bus's gain is one number per bus per frame.
- **The tables as RON**, behind the `serde` feature, with `from_ron`/`to_ron` on `CueTable`, `SirenSettings` and
  `MusicScore`, and a `problems()` on each that returns sentences a person reads.
- `examples/mixer` — the audio panel with its capture knobs, and `examples/music` — a scripted calm → combat → calm
  that checks every transition landed on a bar.

[Unreleased]: https://github.com/bevy-coven/bevy_siren/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bevy-coven/bevy_siren/releases/tag/v0.1.0
