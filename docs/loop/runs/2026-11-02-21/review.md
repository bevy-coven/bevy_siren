# review.md — iteration 21, three passes

## Pass 1 — function

**The claim.** `CueTable::problems()` is a wrapper over `problems_with(&DEFAULT_BUSES)`, documented as *"against the
crate's own four buses"*. A project that lists its own buses — fully supported, and the answer `answer`'s `UnknownBus`
check, `update_mix` and `SirenSettings::problems()` all read — gets one false `UnknownBus` per cue, and the Cauldron's
audio panel draws that call.

| row | source | before | after |
|---|---|---|---|
| a cue on a custom `Sfx` bus | S1 (`bus_names()` is the one answer), S2 (R6 — the report names what the play decides) | `problems()` → **false `UnknownBus`**; the panel draws it | `problems_in(&settings)` → **empty**; the panel calls it |
| the same table with no buses listed | S1 | both agree — the crate's four | **unchanged** (the guard) |
| a `gain_db: NaN` on a cue or slot | S3 (iteration 18's decision file) | 0 sentences | **unchanged — blocked**, recorded as `cues.yaml` case 31, `state: red` |

**Against the sources.** S1 is the whole argument: four readers ask `bus_names()` and the fifth assumed the defaults. The
fifth is the one an editor calls. Iteration 20 fixed the identical bypass in `SirenSettings::problems()`; this is the last
reader.

**The blocked half is stated, not worked around.** The `CueProblem` variant needs a public-enum decision
(`needs-decision/bad-gain-problem.md`, iteration 18, now carrying this iteration's evidence that two of three numbers in
one struct are covered). `cues.yaml` gains case 31 as a red row so the loop cannot forget it, and the guide stops implying
`problems()` catches a bad gain.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/custom-buses.txt` and `evidence/gain-report.txt`, the spectrograms, the loudness table.

**Audio unchanged, and that is correct.** No `src/` behaviour changed for any play: `problems_in` is a report, and the
gain gap is a report. 21/21 WAVs byte-identical; `mix.wav` −4.3 LUFS / +0.2 dBFS.

**The measured table**, `evidence/custom-buses.txt`:

```
settings.bus_names()          ["Sfx", "Voices"]
table.problems()              ["cue `gunshot` plays on bus `Sfx`, which the mix table does not have"]
table.problems_in(&settings)  []
```

The first line is the proof the cue is legal; the second is the lie; the third is the fix.

**A correction in my own work:** the new test first went into `tests/table.rs`, which is behind `#[cfg(feature = "serde")]`,
so `cargo test --test all table::problems_in` ran **zero** tests and reported success. It passed only because the filter
matched nothing. Moving it to `tests/mix.rs` — which has no feature gate — made it a real test, and against the unfixed
tree it now fails to **compile** (`no method named problems_in`), which is the strongest form of "fails before".

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The test uses a **custom bus name** (`Sfx`) that appears in no other test, and asserts both
directions: the wrapper **must** report it (pinning the documented behaviour) and `problems_in` **must** not (the fix).
The doctest is compiled, not `ignore`d — it showed as `1 ignored` in the first gate run and was rewritten as a real
example.

**Can a developer understand it without the code?** `problems_in`'s doc says which call to want and why; `problems()`' doc
now warns what it assumes; the guide's list of checks says which number it does not cover and what the owner's decision is.

**What a fresh reader would still ask:** *why keep `problems()` at all?* Because a host with no settings — a test, a data
check — wants exactly the default-four answer, and its doc says so. The fault was that it was the call named `problems`.

**Verdict: GREAT.**
