# review.md — iteration 54, three passes

## Pass 1 — Function: does a dropped rule stop holding a start?

**The claim** (Gregory §14.6.3): a duck is a group lowering another group **for a while** — an envelope with an end, which
is why the crate stores a start rather than a volume. An envelope that outlives its rule is not a longer duck; it is a
record of a duck that no longer exists.

**Measured**, `DuckClock`'s map, read through its own `Debug`:

| scenario | before | after |
|---|---|---|
| 2000 `fire`s, then `prune` at t=1000 | **2000 entries** | 0 (no rule names them) |
| 2 fires, table names one, then `retain_rules` | 2 | **1** |
| 100 table edits, table names one | 100 | **1** |
| a rule still in the table across a retain | −8.0 dB | **−8.0 dB, unchanged** |

**The split is the fix.** `prune(now_s)` asked *"is this manual envelope finished?"*, which the clock can answer alone.
`retain_rules(&settings)` asks *"is this rule start still a rule?"*, which only the table can answer. Conflating them was
why one collection was cleaned and the other was not.

**Verdict: GREAT.**

## Pass 2 — Zoom: the non-regression, on the bus gain itself

**The risk.** A retain that dropped a *live* rule's start would restart its hold — the bus would snap back to 0 dB mid-duck
and duck again. That is a worse bug than the leak.

**Read the gain, not the waveform.** `showwavespic` on `bus-Music.wav` rendered **blank**: the file is a gain staircase,
not audio, and the skill says to plot a bus gain as a **line**. `evidence/plots/bus-music-gain.svg` is that line, drawn
from the WAV's own samples (40 distinct steps over 4.0 s):

- **0.00 dB** flat until **1.00 s**, when the duck fires (marked);
- a staircase down to **−7.96 dB** at **1.27 s** — the 0.25 s hold plus 0.02 s into the fade;
- a smooth recovery, **complete by 1.97 s** (hold 0.25 + fade 0.60 + the crate's easing).

**No restart, no step, no second duck.** The curve is the same one iteration 49 measured, and `render.txt` still reads
`duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`.

**A colour mistake caught by looking.** My first plot used `var(--accent)` for the path; the rasterizer drew nothing, so
the picture showed axes and no curve. The theme's colours resolve in the reader's viewer but not in every renderer, so the
final plot uses literal values and reads in both.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: is the cost real, and is the leak really reachable?

**The cost, measured rather than argued.** `retain_rules` runs every frame, so it is new per-frame work. 200 000
iterations per case: **14.3 ns / 13.9 ns / 13.0 ns** at 1 / 10 / 1000 held starts — flat, because the map is bounded by
the table after the first frame. Three orders of magnitude below a 16.7 ms frame.

**Reachability, checked two ways.** `DuckClock::fire` is public, so a host can push any pair. The crate's own path is
`fire_all(&settings, &started.bus, now)`, which fires only rules in the **current** table — so the leak grows when a
table's rule set *changes*, exactly what the Cauldron does on every edit of `family/bevy_siren.ron`.

**What I got wrong last iteration.** Iteration 53 logged this as *"unobservable — no accessor, so no capture could prove
it"*. It was observable all along: `DuckClock` derives `Debug`. The accessor was worth adding for a panel and a test, but
the claim that nothing could prove it was false, and it delayed a one-line fix by an iteration.

**Verdict: GREAT.**
