# plan.md — iteration 6: a refused sample names the sound, not the cue

## The pick, and why it beats the runner-up

**Pick:** `start_pending` reports a **decode failure** as `MissingSound { sound: voice.cue }` — the cue's name in a
field documented as the sound *key*. A broken `step_03.wav` tells a project to look for a file called `footstep.grass`.

**Runner-up:** a `Refusal` payload that names which cap refused (`needs-decision/refusal-names-its-cap.md`). That waits
on an owner decision; this one needs no public change.

The pick wins on the loop's own standard: a refusal is the most valuable bug because it is silent, and this one is
worse — it names the wrong thing with confidence, in the crate whose refusals exist to name what is missing.

**Carved off:** a dedicated `Undecodable` variant that lets a host tell "never mapped" from "corrupt file". That is a
public enum change, so `needs-decision/undecodable-sound.md` carries it; the sentence and the key ship now.

## Design

`src/playback.rs`:

- `start_pending` gains `table: Res<CueTable>` and derives the key it should have been naming:

  ```rust
  // The key the cue chose, exactly as `answer` chose it. A table edited between the play and the load can no longer
  // answer; the cue's name is then the only true thing left to say, and the sentence says so.
  let key = table
      .get(&voice.cue)
      .and_then(|cue| cue.sounds.get(voice.variant))
      .map(|slot| slot.sound.clone());
  let reason = match key {
      Some(sound) => Refusal::MissingSound { sound },
      None => Refusal::UnknownCue,   // the cue left the table while its bytes loaded
  };
  ```

  `UnknownCue` is already a variant and is exactly true in that case — the cue is not in the table — so no new variant.
  The `Entry::Refused` line and the `Refused` message both carry the corrected reason.

`src/play.rs`:

- `MissingSound`'s doc and `Display` become true for both of its causes:
  *"no handle or undecodable bytes for sound `{sound}`"*, doc *"the cue's chosen variant names a `sound` key with no
  handle, or one whose bytes the decoder refused"*.

`examples/budget.rs`:

- the probe gains a **missing key** case beside the existing bad-bytes case, so `refusals.txt` shows the two
  `MissingSound` causes naming two different keys.

## Every field touched on a serde type

None. `MissingSound.sound` keeps its name and type; no `CueTable`, `SirenSettings` or `MusicScore` field moves; no new
`Refusal` variant. The only public change is one `Display` string and one doc sentence.

## Tests that prove it

`tests/mixer.rs` (the feature-gated backend, which is where `start_pending` runs):

1. `a_sample_the_decoder_refuses_names_its_sound_key` — a cue `footstep.grass` whose variant key is `step_03`, with a
   real handle to four zero bytes; the `Refused` reason is `MissingSound { sound: "step_03" }`, not the cue. **Fails on
   the unfixed tree**, which says `footstep.grass`.
2. `a_key_with_no_handle_still_names_the_key` — the other cause is unchanged: no handle for `step_03` is still
   `MissingSound { sound: "step_03" }`, produced by `answer`. This must **not** regress.
3. `a_cue_that_left_the_table_before_its_bytes_loaded_is_unknown` — the cue is removed after the play and before the
   asset resolves; the refusal is `UnknownCue`, which is true.

`tests/play.rs`:

4. `the_missing_sound_sentence_covers_both_causes` — one assertion on the exact string, so a panel's line and the
   variant cannot drift.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (cases 10–13 are the other refusals and must stay green);
- `cargo run --release --example perfcheck` before/after — the change adds one table lookup per **waiting** voice per
  frame, which is a real cost on the mixer path and must be measured, not assumed;
- `examples/budget`'s `refusals.txt` before/after: the bad-bytes line names a key, and the missing-key line names the
  same shape of key;
- `examples/render`'s cue WAVs byte-identical (no sample changes).

## Risk

`start_pending` now reads `CueTable` every frame it has waiting voices. A waiting voice is one whose bytes are still
loading — normally one or two frames — so this is a `Res` read plus a `Vec` scan per waiting voice, not per voice. The
alternative (storing the key on the voice) is a public field, which is a bigger change for less: the key is derivable
and the derivation is the same expression `answer` used.
