# Should a cue with an exact pitch range spend a draw from the shared stream?

**Status:** open — found in iteration 60 by probing; **not fixed**, because fixing it moves every seeded stream.

## What the crate does now

`answer` computes the cue's detune unconditionally:

```rust
let mut pitch = cue.pitch.at(choices.rng.next_f32());   // always draws
if let Some(extra) = request.pitch { pitch += extra; }
pitch = crate::model::played_semitones(finite_or(pitch, 0.0));
```

`PitchRange::exact(n)` is `{ low: n, high: n }`, and `at` is `low + (high - low) * unit` — with `low == high` the unit
cannot change the answer. So the draw is **provably useless**, and it is spent from `SirenChoices::rng`, the one stream
every cue shares.

**Measured**: an app that plays an exact-pitch cue four times before a `Shuffle` cue produces a **different** variant
sequence for that second cue than an app that never played the first. Six plays, seed default, two sounds.

## Why this matters

`SirenSettings::seed` is the project's reproducibility lever (`docs/guide/cues.md`: *"Two projects that differ only in
`seed` play different variants"*, and the determinism test asserts the same seed replays field for field). A draw that a
cue cannot use means **adding or removing a cue from a scene changes what every other cue plays** — a project's
soundscape is not a function of its seed alone, but of its play *order* including cues whose pitch is fixed. That is
worth knowing, and it is exactly the kind of coupling a seeded stream exists to remove.

## The options

### A — keep it; say so

- **Cost:** none; the stream stays as it is.
- **Benefit:** every existing project's stream is unchanged, byte for byte.
- **What it costs a reader:** the guide must say that a play draws for its detune **whether or not** its range is exact,
  so "add a cue to the scene" is documented as shifting the stream.

### B — skip the draw when `low == high`

```rust
let mut pitch = if cue.pitch.low == cue.pitch.high { cue.pitch.low } else { cue.pitch.at(choices.rng.next_f32()) };
```

- **Cost:** a **behaviour change**. Every project with an exact-pitch cue sees its subsequent variants move: the stream
  is the same seed but a different sequence of draws. In a **shipped** game that is an audible difference on the next
  build, for no authoring change.
- **Benefit:** the stream depends only on the plays that needed a number, so a scene's soundscape is a function of its
  seed and its cue set, not of how many fixed-pitch cues ran before it.
- **This is the option whose blast radius is why the fix is not an iteration's call.**

### C — a separate stream for pitch and for variants

- **Cost:** a second `Rng` on `SirenChoices`, and every project's stream changes once — more than B, not less.
- **Benefit:** the two uses cannot perturb each other at all.

## The recommendation

**B, at the next minor.** It is the honest arithmetic — a draw that cannot affect the answer is not a draw — and the
same argument iteration 51 made for a picker window that was the allocator's rather than the author's. The reason it is
not shipped today is not doubt about the bug; it is that the fix **silently re-rolls every project's soundscape**, which
is a release note, not a patch.

## What is blocked

Nothing mechanically. The crate keeps drawing, and this file is the record so the change happens deliberately with its
release note instead of as a side effect.
