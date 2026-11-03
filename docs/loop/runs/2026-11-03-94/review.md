# Iteration 94 — review

## Pass 1 — Function: are #27's remaining two rules met?

**Critique.** Two rules, and they differ in kind exactly as `#27` says they should:

| Rule | Kind | Threshold | Why |
|---|---|---|---|
| tempo | **hard exclusion** | **4.0 BPM** | 90 under 128 is a clash whatever the timbres — the issue's own example, refused at 38 BPM apart |
| key | **penalty only** | `Compatibility`'s 0.5 / 0.85 | a modal piece sits anywhere, and key is the least reliable feature here |

**The tolerance is derived, not chosen.** The estimator carries ±2 BPM of uncertainty (ADR 0003's floor), so a 4 BPM
tolerance is **twice** it: below that, a difference may be entirely measurement error, and excluding on it would reject
tracks whose tempi merely look different — the false-positive cost `#27` calls worse than a weak ranking.

**And the boundary is asserted on both sides**: 3.9 and **4.0** admitted, **4.1** and 38.0 refused. A filter off by one
comparison survives every visual check.

**Verdict: GREAT.**

## Pass 2 — Zoom: the distinction iteration 93 made storable

**Critique.** Iteration 93 stored three states — a tempo record with a `bpm`, a record with `bpm: None` (*judged, not
rhythmic*), and no record (*never measured*). **Reading that wrong is the failure mode of this iteration**, and it is
easy to write:

```rust
// WRONG: a non-rhythmic asset contributes None, and every such asset then compares equal.
candidate.tempo.map(|t| t.bpm.unwrap_or(120.0))
```

`Features::bpm()` returns `tempo.and_then(|t| t.bpm)`, so a record whose `bpm` is `None` reads as **no tempo** and the
rule is silent. The test `an_unknown_or_non_rhythmic_tempo_never_excludes` asserts all four cases: absent record,
`bpm: None`, and each on either side.

The key rule has the same shape one level down — `found_key()` filters `Key::is_some()`, so a **judged-atonal** record
penalises nothing — and `an_unknown_key_never_penalises` asserts it separately, because *absent* and *judged atonal* are
different records that must behave identically here.

**And `multiplier == 1.0` records no penalty.** A project that sets `clash_penalty: 1.0` should see the rule stop, not a
penalty whose value is nothing — `the_new_thresholds_are_overridable` asserts that.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a project disagree with the policy?

**Critique.** Every number is on `Weights`, which a project ships as RON:

```rust
tempo_tolerance_bpm: 4.0,        // twice the ±2 BPM estimator floor
compatibility: Compatibility {   // the named table #27 asked for, from #25
    clash_penalty: 0.5,
    parallel_penalty: 0.85,
},
```

A stylised world that *wants* tempo jumps raises the tolerance; one that wants clashing keys sets `clash_penalty: 1.0`.
Neither needs a flag, because the number is the flag.

**Both fields are validated.** A negative `tempo_tolerance_bpm` would exclude every pair, and a multiplier outside
`0..=1` would either reward a clash or invert the score; both are reported rather than obeyed. The defaults are asserted
to report nothing, so a project shipping them is not warned.

**What is deliberately not done**: `Tempo::octave_ambiguous` is stored and **not read** by the rule. An octave-ambiguous
estimate against its own octave is 60 BPM apart at 120, well outside the tolerance either way — so reading the flag would
need a rule about *which* octave to compare in, which is a design question rather than a bug. Named as carved off.

**Verdict: GREAT.**
