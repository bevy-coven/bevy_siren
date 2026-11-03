# research.md — iteration 56: a sound key the table dropped stops holding its handle

## The item

`SirenSounds` is the key → `Handle<AudioSource>` map a project fills by hand or with `load_table`. It exposes
`insert`, `get`, `len`, `is_empty`, `keys`, `load_table`, `missing`, `failed` — and **no remover of any kind**.

`load_table` **merges**:

```rust
for key in table.sounds() {
    if self.handles.contains_key(key) { continue; }
    self.handles.insert(key.to_owned(), server.load::<AudioSource>(path));
    asked.push(key.to_owned());
}
```

It skips a key it holds and never drops one the table has lost. So a project that reloads a table with renamed keys —
the ordinary shape when an editor re-reads the definitions file, or when a project swaps a level's cue set — keeps one
handle per key it has **ever** loaded, for the life of the process.

**Measured**: two tables of ten different keys, loaded in turn, leave **twenty** keys held.

## Why the handle is the cost, not the map entry

A `Handle<AudioSource>` is **24 bytes**; the map entry is the key `String` (**24 bytes**) plus bookkeeping — about
**62 KiB for a thousand stale keys**. That is not the problem.

**The handle is what keeps the asset alive.** `AudioSource` is:

```rust
pub struct AudioSource { pub bytes: Arc<[u8]> }   // bevy_audio-0.20.0/src/audio_source.rs:19
```

— **the whole encoded file**. A strong `Handle` in a live map means the asset stays loaded, so a thousand stale keys are a
thousand audio files resident. For a game with a few hundred MB of audio, that is the difference between swapping a
level's sounds and never releasing them.

## Reachability

**`src/lib.rs:24`'s own quick start calls `load_table`** (`sounds.load_table(&assets, &cue_table, "audio")`), and the
Cauldron's `audio_panel_reads_the_crates_file` test drives `SirenSounds` directly. So the merge path is the documented
one, not an edge.

**Purely a memory bug, not a wrong-output one** — checked rather than assumed: `missing()` and `failed()` both iterate
`table.sounds()`, and every `sounds.get(...)` in the crate is called with a key taken from the current table
(`src/play.rs:538`). A stale key is never read, so it cannot make a refusal lie or a cue play the wrong file. That is why
this sits below the blocked report items rather than above them.

## Sources

**S1. Böttcher, *Procedural Audio* (2008)** — *"the main problem of procedural audio is not the synthesis but the
**management of thousands of assets**"* (`docs/research/siren.md:113`). This module's own doc quotes it: *"the *asset
workflow* is the hard part, so the crate keeps the mapping in one place"*. A map that grows without bound is that
workflow failing at the one job the module exists for.

**S2. `AudioSource::bytes: Arc<[u8]>`** — bevy_audio 0.20.0, read in the vendored registry source. The encoded file,
held whole, kept alive by a strong handle. The number that makes the bug worth a sentence.

**S3. `load_table`'s own doc** — *"returns the keys it asked for that it had no handle for yet"*. Its contract is
**ask for what is missing**; dropping what the table lost is a different question, which is why the retain is a separate
method rather than a line inside it. Changing `load_table` to remove would silently change what an existing caller's
map means.

**S4. The crate's own precedent, twice over** — `DuckClock::retain_rules` (iteration 54) and
`SirenChoices::retain_cues` (iteration 55): *the table owns the names, so the table decides what is retained.* This is
the third and last map of that shape.

## The technique

`SirenSounds::retain_keys(&table)` drops every key the table no longer names, in one pass over `table.sounds()` (a `Vec`,
so no nested scan). A caller reloads as `retain_keys(&new_table)` then `load_table(.., &new_table, ..)`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `Handle<AudioSource>` | **24 bytes** | measured |
| map entry overhead, 1000 stale keys | **~62 KiB** | measured |
| a stale handle's real cost | **one whole encoded audio file, resident** | S2 |
| keys held after two 10-key tables, before | **20** | measured |
| after `retain_keys` | **10** | measured |
| whether a stale key can be read | **no** — every `get` uses a current-table key | measured, `src/play.rs:538` |

## What good sounds like

Nothing audible changes — this is memory, not sound. What a developer sees is `SirenSounds::len()` meaning *the keys the
table names*, a reload that releases the previous level's audio, and `missing()` still naming exactly the files to
deliver. On the panel, the same cues and the same plays.

## Blocked

`hs distill search` answered **`500 Internal Server Error: Embedding error: gpu busy: yielded to bevy_grimoire@big`**,
the sixth iteration running. The citation here is the vendored `docs/research/siren.md` and the registry source; no new
DOI bears on a map that is never emptied.
