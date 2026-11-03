# Should an empty `sound` key be a `CueProblem`?

**Status:** open — found this iteration, not fixed.

## What the crate does now

```
( cues: [(name: "blank", sounds: [(sound: "")])] )
  from_ron accepts it;  problems(): []
  a play is answered Played with `sound: ""`
```

`Cue::name` and `BusLevel::name` are both checked with `trim().is_empty()` in `problems()`; a **slot's `sound` key** is the
only name in the crate that is not. The play is then answered `Played` naming a key no file can have, so a host looking
for the file to deliver is sent to `audio/`.

## Options

### A — `CueProblem::EmptySound { name: String }`

- **Cost:** the same public-enum break as `bad-gain-problem.md` and `duplicate-sound-problem.md`. Three variants waiting
  on one decision.
- **Benefit:** the third name check the crate already applies to every other name.

### B — fold it into `NoSounds`

- **Cost:** `NoSounds`' own sentence is *"has no sounds, so it can only be refused"*, and a cue with `[""]` is **not**
  refused — it plays. The sentence would lie.
- **Benefit:** no new variant.

### C — nothing

- **Cost:** an author whose generated table emitted `sound: ""` gets a `Played` for a key that cannot exist.
- **Benefit:** none.

## Recommendation

**A, with the other two variants, in one breaking release.** All three are the same kind of change to the same enum, and
they should cost a consumer one migration, not three. Until then this file records the gap and `cues.yaml` does not carry
it (unlike case 31, this one has no behaviour fix to sit beside).

**What it costs a consumer today:** a table with an empty key plays and is not reported; the cue's `Played.sound` is `""`.
