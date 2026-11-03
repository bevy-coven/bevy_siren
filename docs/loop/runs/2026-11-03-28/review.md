# review.md — iteration 28, three passes

## Pass 1 — function

**The claim.** `Play::on(entity)` parents the voice to a game entity; despawning that entity is the **ordinary** way a
parented sound ends. On Bevy's path the sink is a component on the voice's own entity and dies with it. On the crate's
mixer the voice's number is also a component on that entity — so the despawn destroys the number and nothing ever calls
`stop_voice`.

| row | source | before | after |
|---|---|---|---|
| one parented loop voice, owner despawned | S1 (Bevy puts the sink on the player's entity), S2 (R4: a stopped voice frees its slot), S3 (the two backends answer identically) | `stop_voice` **never called** — the `rodio` player sounds for ever, slot held | `stop_voice [0]` **once**; slot freed; mix counts 0 |
| owner alive for ten frames (guard) | S1 | `stop_voice []` | **unchanged** — `[]` |
| every rendered cue | — | — | **byte-identical** (21/21) |

**Against the sources.** S3 is what makes this rank 1 rather than 3: it is not only a leak but a **divergence**, and the
crate's own rule says the two backends answer identically with only the sample path differing. S2 is the consequence:
a slot nobody can stop is the permanent lockout R4 exists to prevent, arriving by the mixer's own bookkeeping.

**What is *not* fixed and is named:** **neither** backend writes a `stopped` line for a parent's despawn. The log keeps
`played` and nothing else. That is a missing **record**, not a leak, and it needs a new `Entry` reason — the same
public-enum decision five other items wait on.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/mixer-slot{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**The measured table**, `evidence/mixer-slot{,-before}.txt`:

```
before                                        after
started 1, owner alive:  stopped []           stopped []      (guard)
started 1, owner gone:   stopped []           stopped [0]     (the fix)
mix voices: 0                               0
log: ["played engine …"]                     ["played engine …"]   (the record gap, named)
```

**A correction in my own research, made rather than hidden.** Two drafts of `research.md`/`plan.md` argued that the fix
needed a **new public resource** and wrote a decision file instead. Probing further showed a `Local` in `start_pending`
does it with **no public type at all** — the earlier drafts were wrong about what was blocked, and both files were
rewritten before the code was committed.

**A rejected design, recorded:** Bevy 0.20 has a component `on_despawn` hook that would be the obvious place, but its
`DeferredWorld` cannot reach the `Output` **resource**, so it cannot call `stop_voice`. The reconciliation belongs where
the trait object is.

**Verdict: GREAT** after the research correction.

## Pass 3 — fresh eyes

**A different shape.** The test is behind `mixer` — the only configuration where a slot exists — and its **guard is
inside the same test**: ten frames with the owner alive must leave `stopped` empty, then the despawn must add exactly
`[0]`. A reconciliation that stopped everything would fail the first half; one that stopped nothing fails the second.

**Can a developer understand it without the code?** `start_pending`'s `Local` carries a doc comment naming the measured
failure (*"one voice started, the owner despawned, `stop_voice` never called"*), and the `retain` reads as the mirror of
`retire_voices`: one frees a voice whose sample ended, the other a voice the world took. A reader who sees a mixer slot
stay occupied has one place to look.

**What a fresh reader would still ask:** *why is there no log line?* Because a `Stopped` reason is a new `Entry` variant's
worth of decision, and the loop has five of those open; it is recorded as the next item, with the measured evidence that
the voice's entity is gone and `SirenMix::voices` already reads 0.

**Verdict: GREAT.**
