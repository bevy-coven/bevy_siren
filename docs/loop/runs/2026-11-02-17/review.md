# review.md — iteration 17, three passes

## Pass 1 — function

**The claim.** Who owns a voice's playback is a fact about the world — is there a mixer? — and not a public flag on its
own. `spawn_voice` took it from `Backend(true)` alone, so a world with `Backend(true)` and no `Output` gave a voice **no
`AudioPlayer`** and **no mixer**: it could not sound, so nothing ended it, so nothing retired it.

| row | source | before | after |
|---|---|---|---|
| `Backend(true)`, no `Output`, one play | S1 (§14.5.8.4 — a voice is a fixed resource; a stopped voice frees its slot); S2 (no `AudioPlayer` ⇒ no sound on Bevy's path) | `AudioPlayer 0` — silent, unretirable | `AudioPlayer 1`; `Played` answered, voice can end |
| `Backend(true)` with an `Output` | S3 (the mixer owns it; the two paths must not both play one request) | `AudioPlayer 0` | **unchanged** — `AudioPlayer 0` (the guard row) |
| `Backend(false)`, no `Output` | S3 | `AudioPlayer 1` | unchanged |
| a **steal**, both paths | R4 | identical `SirenLog` | unchanged |
| a scripted **duck**, 90 frames | R3 | `bus_db`/`ducked` identical; min −8.000 dB on each | unchanged |

**Against the sources.** S1 is why it ranks 1 rather than 3: a leaked slot is not a missing sound, it is a budget that
never comes back — the exact failure cases 20 and 21 of `cues.yaml` were written to catch, arriving by a route they do
not construct. S3 is the promise the fix keeps: `start_pending`'s doc already said Bevy's `AudioPlayer` owns every voice
without an `Output`, and the spawn never read the resource that sentence is about.

**The two-backend hunt came back clean** on the two questions worth asking — a steal and a duck answer identically. The
defect was one level down: not *do the backends answer the same*, but *can a backend answer nothing at all*.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack}.png`, the loudness table, both probe files.

**Audio unchanged, deliberately.** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS, `footstep-grass`
−18.4 LUFS / −6.1 dBFS (looped past the gating block), `mix.wav` −4.3 LUFS / +0.2 dBFS — every number identical to
iterations 14–16. This is a **state** fix: a sound that used to be silently lost now plays, and no rendered cue was in
that state, so a changed WAV would mean the fix had reached further than it should.

**The measured table**, `evidence/backend{,-before}.txt`:

```
before   Backend(false), no Output:  AudioPlayer 1     after: 1
before   Backend(true),  NO Output:  AudioPlayer 0  -> after: 1
before   Backend(true),  with Output: AudioPlayer 0    after: 0
```

The third row is the one that proves the fix is a **predicate**, not a blanket: a world with a mixer still gets no
`AudioPlayer`, so the two paths cannot both play the request.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The guard test runs **only** behind `mixer` — the only configuration where an `Output` and a
`Backend(true)` can both be genuine — and the fallback test **removes** the `Output` the plugin inserted, because with
the feature on this machine `MixerBackend::open()` succeeds and inserts one. The first attempt asserted
`AudioPlayer 1` without removing it and failed on `--features mixer`: a test whose precondition is "no `Output`" must
say so, not assume the plugin left the world as it found it.

Both halves are needed: the fallback test **fails on the base commit**, and the guard test **passes on both** — the
"must not happen" half of the rule, which a fix like this could break by always inserting a player.

**Can a developer understand it without the code?** The question the test asks is one a reader can check without
code: *does the voice have a player?* `Backend`'s doc already said `true` means *"the crate's own mixer started"* — the
fix is what makes the flag mean that rather than "somebody should have started one".

**What a fresh reader would still ask:** *what if a host inserts an `Output` that is a fake?* Then the world is
consistent — the fake is the mixer, and the crate routes to it. That is the host's choice, and it plays.

**Verdict: GREAT.**
