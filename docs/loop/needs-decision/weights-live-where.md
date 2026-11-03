# Where does a project write its ensemble weights?

## The question

`Weights` exists as of iteration 70 (`src/semantic/weights.rs`) and is not reachable from a project's file. `#41`'s own
technical note says to **nest it in the existing settings type** rather than add a parallel configuration root — *"a
project already edits one file, and a second one is a second thing to forget."*

That is the right instinct, and it is also a **change to the definitions-file schema**, which the crate's rules send here
rather than decide.

## What is already true

- `SirenSettings` is a public `Resource` with `#[serde(deny_unknown_fields)]`, read and written by the Cauldron at
  `<project>/family/bevy_siren.ron`.
- That file already holds three sections — `settings`, `cues` and (since iteration 61) `music`.
- Every field of `SirenSettings` carries `#[serde(default)]`, so **an old file gains a field's default rather than
  failing to parse**. Iteration 70 asserts that promise for `Weights` itself.
- The Cauldron's panel writes that file. A `semantic` block in it would be a fourth section the panel must round-trip or
  it would silently drop it — which is exactly the defect iteration 61 fixed.

## The options

### Option A — a field on `SirenSettings`

```rust
pub struct SirenSettings {
    // … existing fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub semantic: Weights,
}
```

- **Cost:** `SirenSettings` gains a dependency on a type behind the `semantic` feature, so the field must be
  `cfg`-gated — and a `cfg`-gated field on a public struct is a schema that differs between two builds of the same
  crate. A project that reads its file without `semantic` and writes it back would **drop the block**, which is the
  iteration-61 bug in a new place.
- **Benefit:** one file, as the note asks, and an old file keeps loading.
- **Risk:** the gating is the whole problem. `Weights` would have to move out of `semantic` into the always-on half to
  avoid it — and then a crate that knows nothing about retrieval ships a retrieval type.

### Option B — a fourth section of the project file, ungated

The project file becomes `(settings, cues, music, semantic)`, with `semantic` always present, and the Cauldron carries
it the way it now carries `music`.

- **Cost:** every consumer that reads the file must gain a field — this crate's `from_ron` helper if it has one, and the
  Cauldron's reader. A consumer that does not is the iteration-61 defect again.
- **Benefit:** no `cfg` gating, no schema that differs between builds, and one file.
- **Risk:** the Cauldron change is a consumer commit in `bevy_grimoire`, which this repository's rules require to be its
  own change there.

### Option C — its own file, `<project>/family/bevy_siren_weights.ron`

- **Cost:** two files, which the note argues against. A project that copies its `family/` folder and misses one gets
  defaults silently.
- **Benefit:** nothing new on an existing type; the reader is `Weights::from_ron` and nothing else changes.
- **Risk:** the note's objection is correct — a second file is a second thing to forget.

## Recommendation

**Option B**, and it is the only one that does not reintroduce the defect this project has already paid for twice.

The deciding fact is not convenience: it is that **iteration 61 shipped a consumer fix because a reader with fewer
fields than the writer silently deletes data.** RON ignores an unknown key without `deny_unknown_fields`. Option A's
`cfg`-gated field produces exactly that asymmetry between a build with `semantic` and one without, and Option C produces
it between a project that copied two files and one that copied one.

**What ships with it:**

1. A round-trip test in this crate asserting a `semantic` block survives `from_ron` → `to_ron` → `from_ron`.
2. A companion commit in `bevy_grimoire` adding the field to its reader, with the existing lossless-round-trip test
   extended to cover it.
3. The `deny_unknown_fields` question decided **once**: iteration 61 measured that RON ignores an unknown key without
   it, and that is why the Cauldron's reader was the defect. A file with a typo should be a message, not a silence.

## What each option costs a consumer

| | files | schema differs between builds | consumer work |
|---|---|---|---|
| A | 1 | **yes**, if `cfg`-gated | none |
| B | 1 | no | the Cauldron gains a field |
| C | 2 | no | none |

## Why this is not decided here

The crate's rule is that a change to the definitions-file schema goes to `needs-decision/`. The type ships either way;
what is undecided is where a project writes it, and that is a decision about **their** file rather than about this
crate's code.
