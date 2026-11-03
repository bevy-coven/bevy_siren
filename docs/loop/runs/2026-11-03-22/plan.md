# plan.md — iteration 22: avoid the sound, not the slot index

## The pick, and why it beats the runner-up

**Pick:** `VariantPicker` refuses the last **indices**, and a cue that lists the same file twice has two indices and one
sound. Under the crate's **default** `Repeat::AvoidRepeat { avoid: 1 }` such a cue plays `a` **eight times in a row** —
the machine-gun footstep `Repeat` exists to kill — and `problems()` says nothing.

Beats the runner-up (`load_table` joins a key onto a directory with no separator rule, so `../outside` escapes it): that is
real but it is the **convenience loader** for the common case, the module doc says a key is *"never a path the crate
guesses"*, and a wrong path costs a `MissingSound` refusal that names the key. The repeat violation is silent and on by
default.

## The design

1. **`VariantPicker` gains a key history and `pick_from(&[&str], avoid, rng)`** — the same contract as `pick`, except the
   avoided window is made of **keys**. Two slots naming `"a"` are one candidate for repetition: after `"a"` is heard, every
   slot whose key is `"a"` is refused, so `["a", "a", "b"]` can never play `a` twice running.
2. **`pick(len, avoid, rng)` is unchanged**, byte for byte, for every caller and test that exists — it is public, and a
   caller without keys still gets index avoidance. The two share `remember`'s window discipline.
3. **`SirenChoices::pick` calls `pick_from`** with the cue's keys, so the play path gets the fix with no API change.
4. **`problems()` names the duplicate** — a seventh `CueProblem`, which is **blocked** on the public-enum decision
   (`needs-decision/bad-gain-problem.md` is about a different variant; a duplicate key needs its own file,
   `needs-decision/duplicate-sound-problem.md`). The behaviour fix ships; the report waits.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `VariantPicker` (public struct, private fields) gains a private key
history; no public type changes and no method changes.

## Tests

- `a_duplicate_key_never_repeats_under_avoid_repeat` — `["a", "a", "b"]`, 32 plays: **no adjacent equal** and `"a"` and
  `"b"` both appear. **Fails on the base commit** (it plays `a` eight times).
- `a_cue_of_one_sound_still_repeats` — the guard: `["a"]` must keep playing `a`; the fix must not make a one-sound cue
  refuse.
- `pick_still_avoids_indices_without_keys` — `pick`'s existing behaviour is unchanged (the pre-existing test already
  pins this; the new one asserts the two agree for **distinct** keys, which is the no-regression half).
- `a_duplicate_key_still_plays_its_own_sound` — `Played.sound` is the key named, not a neighbour's.

## The carve

- **Not** the `CueProblem` variant: blocked, new decision file.
- **Not** `load_table`'s path rule: a separate item, named in `log.md`.
