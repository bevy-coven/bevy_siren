# research.md — iteration 22: the picker refuses an index, the ear hears a sound

## The item

`VariantPicker` remembers the **variant indices** it has picked and refuses those. `Cue::sounds` is a list of
`SoundSlot`s, and nothing stops two of them naming the same key — a copy-pasted line, a build step that concatenates two
lists, a table generated from a folder scan. Two slots with one key are two **indices** and one **sound**:

```
cue "c" with sounds [(sound: "a"), (sound: "a")], repeat: AvoidRepeat { avoid: 1 }
  played: ["a", "a", "a", "a", "a", "a", "a", "a"]     adjacent repeat = true
  problems(): []

cue "c" with sounds [(sound: "a"), (sound: "b")]
  played: ["b", "a", "b", "a", "b", "a", "b", "a"]     adjacent repeat = false
```

The first is the **machine-gun footstep** the whole `Repeat` field exists to prevent, on the crate's default setting, and
`problems()` says nothing.

## Sources

**S1. Böttcher, *Procedural Audio* (2008)** (vendored: `docs/research/siren.md` S5) — repetition is fought with
**alternative recordings and randomisation**, and the hard part of game audio is *"managing thousands of assets and the
workflow around them"*. Two slots naming one file is an **asset-workflow** mistake — the author believed they had two
recordings — so it is exactly the kind the crate should name, not quietly play.

**S2. `Repeat::AvoidRepeat`'s doc** — *"Random, but never one of the last `avoid` variants — the default, and what kills
the machine-gun footstep."* The guarantee is about what a **player hears**, and a player hears keys. An index is the
crate's internal name for a slot; the ear has no access to it.

**S3. `docs/research/siren.md` S4 (Kaushik 2025)** — shipped games *"pre-generate variation tables and choose at
runtime"*. A variation **table** whose entries repeat a recording is not a variation table; choosing among its rows
cannot produce variation, however clever the choosing.

**S4. `VariantPicker`'s own doc, third paragraph** — *"When every variant is inside the avoided window — a cue with two
sounds and `avoid: 4` — the most recent one is still refused, because repeating it immediately is the one thing the
window exists to prevent."* Two **identical keys** are the degenerate case of that sentence: every variant is the same
sound, so the refusal must be about the sound.

**S5. R4 / `problems()`** — `CueProblem` names six table mistakes. A cue that lists one file twice under a repeat-avoiding
`Repeat` is the seventh kind: the table promises variation it does not have.

## The technique

The picker must avoid **keys**, not indices:

1. **`VariantPicker::pick_from(&[&str], avoid, rng)`** — the same contract as `pick`, but the history is the **keys**
   already heard, and a candidate is refused when its key is in that window. Two slots naming `"a"` are one candidate
   for repetition purposes: after `"a"` is heard, **both** are refused, so the next pick is `"b"`.
2. **`pick(len, ..)` keeps working** — it is public, and a caller with no keys (or a test) still gets index-based
   avoidance. `SirenChoices::pick` uses `pick_from` with the cue's keys.
3. **`problems()` names the duplicate** — but that is a seventh `CueProblem`, blocked (see below).

**The history is a `VecDeque<usize>` today.** Changing it to hold keys would change `VariantPicker`'s shape; a **second**
queue of keys inside the same struct, used only by `pick_from`, keeps `pick`'s behaviour byte-identical for every caller
and test that exists.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| repetition is fought with alternative recordings | — | S1 |
| `AvoidRepeat` refuses the last `avoid` **variants a player heard** | default 1 | S2 |
| a variation table whose rows repeat a recording is not a table | — | S3 |
| when every variant is inside the window, the most recent is still refused | — | S4 |
| a table mistake is named by `problems()` | — | S5 |

## What good sounds like

A cue with `sounds: ["a", "a"]` under `AvoidRepeat { avoid: 1 }` **alternates nothing** — it has one sound, so it plays
`"a"` every time, and that is correct: the crate cannot invent a recording. What changes is that a cue with
`["a", "a", "b"]` never plays `"a"` twice in a row, which today it can, by picking index 0 then index 1. And
`problems()` says the table lists one file twice under a repeat-avoiding repeat, so the author can deliver the second
recording.
