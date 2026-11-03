# Should a gain above 0 dB be a `CueProblem` / a score problem?

**Status:** open — found in iteration 35 while auditing the report's own doc; not added.

## The problem

Neither report names a gain **above 0 dB**, and the mix sums linearly:

```
music layer `c` with gain_db: +12.0   ->  problems(): []
                                          db_to_gain(+12) = 3.98   (4x amplitude)
four such layers on one bus               ->  sum_db = +24 dB into the master
```

`sum_db` does not clip — *"several sources heard at once"* is a linear sum — so a score with several boosted layers adds
**above full scale** at the master, and nothing says so. The same is true of a **cue's** `gain_db`, and of a `Play`'s own
`gain(gain_db)` override.

## Options

### A — report a gain above 0 dB, in both reports

- **Costs:** a **new `CueProblem` variant** for the table (the public-enum break seven items already wait on), and a score
  sentence. Both would fire on a table that **boosts on purpose** — a layer at +3 dB to lift a quiet stem is a normal
  mixing move, and reporting it would be a false alarm on a legitimate file.
- **Benefits:** a score that clips the master is named.

### B — report only the **sum**, not the parts

A score whose layers on one bus sum above 0 dB is a fault; each layer individually is not.

- **Costs:** a sentence in the score's report only (no enum change), and a threshold — `sum_db` of the layers on `Music`
  above `0.0` dB. But layers **crossfade**: two layers at 0 dB that are never both audible sum to +6 dB on paper and are
  never both heard, so the check would fire on the *ordinary* case — a score is built to have one layer up at a time.
- **Benefits:** no enum change.
- **Problem:** the false-positive rate is near 100 % for the crate's own example, where `calm` and `combat` are both
  present with one at `SILENT_DB`.

### C — nothing; the master is the host's

- **Costs:** a score that clips is a host's problem to hear and fix, which is what every mixing console assumes.
- **Benefits:** no false alarm on an intentional boost, no API change.

## Recommendation

**C.** A gain above unity is a **mixing decision**, not an authoring mistake: a boost is legitimate, and a score's layers
are designed to crossfade rather than sum, so any threshold either cries wolf or needs a rule about *which* layers can be
audible together — which is the score's own gain automation, not something a static report can know. The crate's rule is
that a bad cue is refused with a reason; a loud cue is not bad, it is loud.

**What it costs a consumer today:** a clip at the master from several boosted layers is not reported. The master's own
level (`SirenSettings::master_db`) is the tool for it, and a host that mixes by ear or by meter sees it.

## What the crate does say, and why this is adjacent

`dB <= SILENT_DB` **is** silence, on the way in and out — a layer at −200 dB reports `-200.0` while the chain treats it as
`0.0`, which is the documented rule, not a gap. The asymmetry is deliberate: silence has a defined floor, and loudness
above unity does not have a defined ceiling.
