# research.md — iteration 3: the seed a project writes, and the hold a stinger promises

## The items

**A. `SirenSettings::seed` is written, documented, and read by nothing.** `SirenChoices` is `init_resource`d with
`Default`, which is `new(SirenSettings::default_seed())`. No system ever rebuilds it from the settings, so a project's
`seed:` field never reaches the stream. Proved with a throwaway crate: `seed: 7` and `seed: 99` produce **byte-identical**
`Played` streams (variant, gain and pitch), and each repeats itself. So the doc on the field — *"the same seed replays
the same sounds, which is what makes a capture or a test reproducible"* — is true only by accident of everyone sharing
one constant, and choosing a seed is impossible.

**B. `Stinger::duck_s` is documented as a hold and passed as a fade.** `play_stingers` calls
`clock.manual("Music", duck_db, now, 0.0, duck_s)`, whose signature is `(target, by_db, now_s, hold_s, fade_s)`. The
duck therefore starts recovering on the frame the stinger fires: with the default `duck_db: -6.0, duck_s: 1.0` the
music is at −6 dB when the stinger starts, −3 dB halfway through it and 0 dB at its end. The doc on the field says
*"how long the duck holds before it starts recovering"*. A stinger is a punctuation mark the music is supposed to
step back **for**; instead the music swells back underneath it.

Both are the loop's first-ranked class: wrong output, and the number a person wrote does not do what its sentence says.

## Sources

1. **Jason Gregory, *Game Engine Architecture*, 3rd ed., §14.6.3** — the duck rule this crate already cites
   (`docs/research/siren.md` R3; read in home-still in iteration 0, doc id `d0213a22…`). What it establishes for this
   iteration: a duck is a **lower-while-something-plays** relation with a **hold** and a **release**, not a one-pole
   decay. The crate's own `active_duck_db(by_db, since_s, hold_s, fade_s)` is exactly that shape, and both the
   dialogue duck and the stinger go through it — so the stinger's bug is a wrong *argument*, not a missing model.

2. **The crate's own documented envelope** — `src/model.rs::active_duck_db`, pinned by
   `tests/model.rs::a_duck_holds_then_recovers`: full strength while `since < hold_s`, then a linear recovery over
   `fade_s`. The stinger should use `hold_s = duck_s` and a release it declares, not `hold_s = 0`.

3. **`Rng`/`SirenChoices` as the crate's reproducibility contract** — `src/model.rs` (*"Never seeded from the clock
   inside this crate: a sound that cannot be reproduced cannot be debugged"*), `SirenSettings::seed`'s doc, and case
   17 of `docs/loop/cues.yaml` (*"The same seed replays the same sounds, gains and pitches … Equal field for field"*).
   The rule exists; the wiring does not.

4. **The literature server was unavailable for this iteration** and is written down rather than worked around: every
   `hs distill search` call on 2026-11-02 returned `Embedding error: gpu busy: … 4222 MB free < 5000 MB required`. The
   `papers/` directory on this Mac is empty (the corpus is on `big`), so no PDF could be read locally either. Both
   items rest on sources this repository already cites and on its own documented rule, not on a new paper; a new
   parameter (the stinger's release) is marked **design default** and argued below.

## The technique

**A. Seed plumbing, one writer.** The mix table is the project's file; the random stream must be derived from it, not
from a crate constant. The rule that already governs this codebase is *one owner per piece of state*: `update_mix`
owns voice gains, `retire_voices` owns the end of a voice. So `SirenChoices` gets one owner for its seed too — a system
in `SirenSystems::Choose` that rebuilds the stream when `SirenSettings::seed` changes, and does nothing otherwise.
Rebuilding **resets the variant histories**, which is what "a different seed" means; the log gets one line so a person
can see it happened.

**B. A stinger's duck holds for the stinger, then releases.** Three numbers, and the crate already has all three
shapes:

| number | meaning | where |
|---|---|---|
| `duck_db` | how far `Music` is lowered | `Stinger::duck_db`, unchanged |
| `duck_s` | how long it **holds** at that depth | `Stinger::duck_s`, its documented meaning at last |
| the release | how long the recovery takes after the hold | **new field**, default 0.25 s (**design default**) |

The release is a new field on a public message type → **decision file**, not a decision. The *fix* (pass `duck_s` as
the hold) ships without it, using the crate's existing `DEFAULT_FADE_S` (0.12 s, already the release every stop and
every steal uses) as the stinger's release until the field is decided. That keeps this iteration inside the crate's
rules: no new public field without a decision, and the number that ships has a source (the crate's own
`DEFAULT_FADE_S`).

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| duck shape | full for `hold_s`, then linear recovery over `fade_s` | crate rule R3 (Gregory §14.6.3) |
| dialogue duck | −8 dB music, −4 dB effects, hold 0.25 s, fade 0.6 / 0.4 s | Gregory §14.6.3, unchanged |
| a stinger's duck depth | −6 dB default | design default, unchanged |
| a stinger's **hold** | `duck_s`, default 1.0 s | `Stinger`'s own doc, now honoured |
| a stinger's **release** | 0.12 s | **design default**, borrowing `DEFAULT_FADE_S`: "short enough to read as immediate, long enough not to click". A field for it is the decision file. |
| seed | any `u64`; 0 is mapped to a constant by `Rng::new` | crate rule: a zero seed would make xorshift produce zero forever |
| default seed | `0x5EED_51AE_0000_0001` | unchanged |

## What good sounds like

**A.** Two projects with different `seed:` values play different variants and different detunes from the first play;
the same `seed:` played twice, in two processes, plays the same sequence field for field. A reviewer can judge it
without hearing it: two `Played` streams side by side, equal within a seed and different across seeds, and a log line
that says the stream restarted because the seed changed.

**B.** A stinger's duck reads as a **step down, a held floor, and a step back up**. On a plot of `Music`'s gain against
time it is a flat-bottomed well as wide as the stinger, not a sawtooth that starts climbing on the same frame it
starts. Measured on the crate's own envelope at 60 fps:

```
hold = duck_s (fixed)          hold = 0 (today)
t = 0.00 s   −6.00 dB          −6.00 dB
t = 0.25 s   −6.00 dB          −4.50 dB
t = 0.50 s   −6.00 dB          −3.00 dB
t = 0.75 s   −6.00 dB          −1.50 dB
t = 1.00 s   −6.00 dB → release 0 dB
```

The left column is a stinger; the right is a swell back into the music while the stinger is still sounding.
