# plan.md — iteration 4: a crossfade that takes the time it was given

## The pick, and why it beats the runner-up

**Pick:** `MusicLayers::fade_s` is stored and read by nothing; every layer move eases at `MIX_TAU_S`, so an authored
800 ms crossfade arrives in ~150 ms and nothing says so.

**Runner-up:** a `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed, so a panel
prints the wrong culprit. Rank 3 (authoring ergonomics) and it needs a `Refusal` shape change — a decision file — so
the whole item waits.

The pick wins on the loop's second criterion (a core behaviour a game needs: music transitions and fades) *and*
because the field is already public and already documented: the crate is silently discarding a number a project
supplied. It is the same failure as iterations 2 and 3, and it needs no new public API.

**Carved off:** the refusal's wording (decision file, next item) and the `Completes` removal (already a decision file).

## Design

`src/music.rs` only. No new field, no new message, no new type.

- `move_layers` computes each layer's time constant from the fade that layer was last given:

  ```rust
  // The crate's meaning of a fade duration is "the time to arrive" (Duck::fade_s + active_duck_db), and `approach`
  // reaches 1 - e^(-t/tau) of the way. So an arrival in `fade_s` within one part in a thousand is tau = fade_s / ln(1000),
  // and a fade of 0 or less keeps MIX_TAU_S, which is what the builder's default promises.
  fn tau_for(fade_s: f32) -> f32 { ... }
  ```

- `LiveLayer` gains `fade_s: f32` — **private**, initialised to `0.0` at start (today's `MIX_TAU_S` easing) and
  replaced when a due `Pending` names the layer. A layer missing from a change keeps its previous fade, exactly as it
  keeps its gain.

## Every field touched on a serde type

None. `MusicLayers::fade_s` already exists with its default (documented) and its builder; this iteration makes it
do what its doc says. No `CueTable`, `SirenSettings` or `MusicScore` field moves.

## Tests that prove it

`tests/music.rs`:

1. `a_layer_takes_the_fade_it_was_given` — at 120 bpm, `fade_s(0.8)` on a layer from `SILENT_DB` to −9 dB: **not** at its
   target at 0.3 s (where today's code has long settled), and within 0.05 dB of it by 0.8 s + two frames. Fails on the
   unfixed tree at the 0.3 s assertion.
2. `a_layer_fade_of_zero_keeps_the_crates_easing` — `fade_s(0)` settles in ~150 ms, so the builder's documented
   default is unchanged.
3. `a_layer_missing_from_a_change_keeps_its_fade` — two layers; move one with a long fade; the other's arrival time is
   unaffected, and a later move of the second with the default uses `MIX_TAU_S`.

`tests/model.rs` — a pure test for the derived constant, so the arithmetic is pinned where the crate's other numbers
live:

4. `a_fade_arrives_within_one_part_in_a_thousand` — `approach` at `tau = fade_s / ln(1000)` is within 0.1 % of the
   target at `fade_s`, and **not** within 0.1 % at `fade_s / 2`. This is the derivation in `research.md` as an
   assertion, not a comment.

`examples/render` gains a second music plot: the same layer move under `MIX_TAU_S` and under a `fade_s(0.8)` time
constant, drawn as two labelled lines, so the evidence is a picture and a later regression shows as the pair
converging.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 15 is the bar quantisation and must stay green; the new
  case 23 is the fade);
- `cargo run --release --example perfcheck` before/after at 8/64/256 — the change adds one `ln` per layer per frame,
  which is below the noise, but must be measured rather than assumed;
- `examples/render` before/after: cue WAVs must be byte-identical (no cue gain moves), and the new plot must show the
  two arrivals;
- `examples/music` unchanged, every transition still on a bar.

## Risk

`LiveLayer.fade_s` is per layer, so a change that moves three layers with three durations is honoured per layer. The
one behaviour worth naming: a **second** change arriving while the first is still easing replaces the fade *and* the
target — the layer does not queue. That is what today's code does with the target, and queueing would be a new
feature with its own decision; the log names it.
