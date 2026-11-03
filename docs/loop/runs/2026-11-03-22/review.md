# review.md — iteration 22, three passes

## Pass 1 — function

**The claim.** `VariantPicker` refused the last **indices**; a cue that lists one file twice has two indices and one
sound. The guarantee `Repeat::AvoidRepeat` makes — *"never one of the last `avoid` variants … what kills the machine-gun
footstep"* — is about what a **player hears**, and a player hears keys.

| cue, `AvoidRepeat { avoid: 1 }` | source | before | after |
|---|---|---|---|
| `["a", "a", "b"]`, 16 plays | S1 (repetition is fought with alternative recordings), S2 (`AvoidRepeat`'s doc), S4 (the degenerate case of its own paragraph) | **3** adjacent repeats — `… b a a a b` | **0** |
| `["a", "a", "a", "b"]` | S3 (a variation table whose rows repeat a recording is not a table) | **8** — `a a a a b a a a` | **0** |
| `["a", "b"]` (control) | — | 0 | **0** — unchanged |
| `["a"]` (one sound, guard) | S4 | 15 | **15** — unchanged; the crate cannot invent a recording |

**Against the sources.** S2's own doc says the window exists to stop *"repeating it immediately"*; two slots naming one
file are the degenerate case, where every candidate is the same sound. S3 is the asset-workflow reading: the author
believed they had two recordings, and a table whose rows repeat one recording cannot produce variation however the rows
are chosen.

**The severity:** this is the crate's **headline** feature — the field that exists so a footstep is not a machine gun —
defeated by a copy-pasted line, on by default, in silence.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/duplicate-keys{,-before}.txt`, the spectrograms, the loudness table, five perf runs.

**Audio unchanged for every table without a duplicate — by construction.** `pick_from` and `pick` agree whenever keys are
distinct, and the test `pick_without_keys_still_avoids_indices` proves it over 16 draws from one seed. 21/21 WAVs
byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS, `footstep-grass` −18.4 LUFS / −6.1 dBFS, `mix.wav` −4.3 LUFS /
+0.2 dBFS — identical to iterations 14–21.

**The measured table** is the whole argument:

```
before                                            after
["a","a","b"]        adjacent repeats 3     ->    0
["a","a","a","b"]    adjacent repeats 8     ->    0
["a","b"] control    adjacent repeats 0     ->    0
["a"]     guard      adjacent repeats 15    ->    15
```

**A perf correction, recorded rather than smoothed.** The first run read 0.407 µs at 256 voices — 2.5× faster than
iteration 21 — which a change that *adds* work cannot cause. Four more runs gave 0.393 / 0.597 / 1.305 / 1.341 / 1.350:
a **3.4× spread on one binary**, load average drifting 8.6 → 16.5. `perf.md` reports all five and the median, not the
flattering first sample.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The tests use a **three-slot cue with a duplicate** — a shape no other play test has — and the
guard is the opposite extreme, a one-sound cue, so a fix that refused everything would fail it. The agreement test seeds
two pickers from one `Rng::new(7)` and asserts the index-based and key-based rules pick the same sequence for distinct
keys, which is the no-regression half a change like this needs.

**Can a developer understand it without the code?** `pick_from`'s doc says *"the avoided window is made of keys, not
indices"* and gives the reason in one line; `SirenChoices::pick` says why it calls it. `Repeat`'s doc is unchanged
because it was always describing the right thing.

**What a fresh reader would still ask:** *why is a duplicate not a `CueProblem`?* Because that is a seventh variant of an
exhaustive public enum; `needs-decision/duplicate-sound-problem.md` records it, recommends reporting **only** under
`AvoidRepeat` (so `cues.yaml` case 3 — one variant, `Shuffle` — stays silent), and says what a host cannot see today.

**Verdict: GREAT.**
