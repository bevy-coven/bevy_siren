# review.md — iteration 58, three passes

## Pass 1 — Function: does a layer change that names nothing say so?

**The claim** (`src/lib.rs:50`, the crate's own contract): *"nothing plays with no explanation, because a sound that does
not happen with no explanation is the hardest bug in a game to find."*

**Measured**, `MusicLayers::new(vec![("nope", -6.0)])` against a two-layer score:

| | log |
|---|---|
| before | **`["0.00s played `track.calm` (calm) on Music at -80.00 dB"]`** — nothing about `nope` |
| after | **`refused `nope`: this score has no layer `nope`, so the change names nothing`** |

`MusicLayers` was the one message in the crate with **no failure path at all**: no `Refusal` arm, no log, no counter. A
`find` that returns `None` inside an `if let` reads as *"apply it if the layer exists"*, which is true and hides that the
caller's whole intent was discarded. The guard had to be an `else`.

**Verdict: GREAT.**

## Pass 2 — Zoom: the shape, and what it must not break

**One lookup, not two.** `if let Some(layer) = find(..)` became `let Some(layer) = find(..) else { … continue }`, so the
same `find` decides both branches and a miss costs no second scan. The happy path does exactly the work it did before.

**A `Refused` entry, not a new variant.** The score is not the mix table, so `UnknownBus` would be a false sentence — and
a new enum variant is the break three open decisions already wait on. This is iteration 9's route for the stinger's
wrong bus, for the same reason.

**The eviction rule helps here.** `SirenLog::push` documents that *"a refusal outlives a play"*: when the ring is full the
oldest **routine** entry goes, so a busy scene cannot scroll this sentence away behind footfalls.

**One change with both cases, tested.** `a_typo_says_which_layer_and_a_real_one_still_moves` sends `combat` (real) and
`cmobat` (typo) in **one** message and asserts `target_db("combat") == Some(-3.0)` **and** that `cmobat` is named — so
the fix cannot pass by refusing wholesale, which is the mistake this kind of change invites.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer see it without the code?

**Yes, and it took three captures to get there — each failure informative.** The panel shows the newest **six** log
lines, and the layer step fires on the bar:

1. **96 frames** — the bar at 160 bpm is 1.5 s = 90 frames, so the step landed at the very end and had not scrolled in.
2. **200 frames** — the line fired but was pushed out behind later plays (`combat -3.0 -> -3.0` visible, sentence gone).
3. **`SIREN_KEYS=layer SIREN_KEY_EVERY=200 SIREN_FRAMES=400`** — the step fires **once**, so the line is still in the
   window.

`docs/media/mixer-layer-typo.png` now shows, on one screen:

```
combat -3.0 dB
3.00s refused `cmobat`: this score has no layer `cmobat`, so the change names nothing
1.50s refused `cmobat`: this score has no layer `cmobat`, so the change names nothing
```

— the real layer moved **and** the typo named, in the same frame of reference. A reader needs no code.

A `SIREN_LAYER=typo` mode was added to `examples/mixer` for exactly this, following `SIREN_MIX=broken`/`zero`'s precedent:
a sentence a capture cannot show is a sentence that is not proved.

**Nothing audible moved**: `bus-Music` still reads `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`, `mix.wav` still
integrates −18.6 LUFS at −6.1 dBFS, and the music cue's spectrogram is a clean attack with silence after 0.85 s. The
change did nothing before and does nothing now — it only **says** so.

**Verdict: GREAT.**
