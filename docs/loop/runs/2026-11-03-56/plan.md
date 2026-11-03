# plan.md — iteration 56: a sound key the table dropped stops holding its handle

## The pick, and why it beats the runner-up

**Pick:** `SirenSounds` has **no remover at all**, and `load_table` merges — two ten-key tables loaded in turn leave
**twenty** keys held, each a strong `Handle<AudioSource>` keeping a whole encoded file resident.

Beats the runner-up (`AvoidRepeat`'s unreported window) only in that it is **not blocked**: that one needs the open
`CueProblem` enum decision, and this needs no API break at all. It is honestly a **memory** bug, not a wrong-output one —
checked, not assumed: `missing()`, `failed()` and every `sounds.get` use keys from the current table, so a stale key can
never be read.

## The ranked list

1. **`SirenSounds` never releases a key; the handle keeps the file resident.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. `reset_histories` has no caller — a consumer item for `bevy_grimoire`.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; a lower default is a decision file.

## The design

- **`retain_keys(&table)`** — drops every key the table no longer names, one pass over `table.sounds()`. Additive.
- **`load_table` is unchanged.** Its contract is *"returns the keys it asked for that it had no handle for yet"* — ask
  for what is missing. Dropping what the table lost is a different question, and folding it in would silently change what
  an existing caller's map means (a stale key is inert, so a project that relied on holding it would lose nothing it
  could observe — but the change would be unannounced).
- **Call order documented**: `retain_keys(&new)` then `load_table(&new)`, so a kept key is dropped and re-asked rather
  than a lost file being held across the load.

## Fields

**No new field, no schema change.** One additive public method.

## Tests

- **`tests/play.rs::a_sound_key_the_table_dropped_does_not_keep_its_handle`** — ten `first_` keys, retain against a
  ten-key `second` table, `len() == 0`; then ten `second_` keys, `len() == 10` not 20. Fails on the old tree (no method).
- **`tests/play.rs::load_table_merges_and_retain_is_the_fix`** — records `missing()` naming the ten new keys before and
  after the retain, so the fix cannot break the report a project acts on.
- **`tests/play.rs::a_second_table_leaves_the_first_tables_keys_held`** — the merge itself, asserted with only
  pre-existing API (`len()`), so it runs on **both** trees: 20 keys for two ten-key tables.
