# The semantic layer's configuration

Every number the semantic layer decides with is documented here, because `Weights` is a serde type a project ships in
its own file: a user cannot edit what is not written down.

The file is the same RON a project already uses for its cue table and mix table. `#[serde(default)]` is on every field,
so a field added later takes its default in an older file rather than failing to parse — a promise, not an accident — and
`deny_unknown_fields` means a typo is refused rather than silently ignored.

```ron
(
    semantic: 1.0,
    acoustic: 0.5,
    tempo: 0.35,
    key: 0.35,
    loudness_tolerance_db: 6.0,
    noise_floor_tolerance_db: 12.0,
    tempo_tolerance_bpm: 4.0,
    compatibility: (clash_penalty: 0.5, parallel_penalty: 0.85),
    drop_fraction: 0.0,
)
```

```rust
let weights = Weights::from_ron(text)?;
for problem in weights.problems() {           // "read it and it is impossible"
    eprintln!("{problem}");
}
```

`from_ron` answers *"I could not read this"*; `problems()` answers *"I read it and it is impossible"*. A caller needs
both, because a file can parse and still ask for something no ranking can do.

## The four ranking weights

Each is a **share** of the ensemble, and `Weights::shares()` normalises them so a result can report *"0.42 of the score
came from the words"*.

| field | default | effect at its extremes | why that default |
|---|---|---|---|
| `semantic` | `1.0` | `0.0` ranks by production alone — legitimate, and `problems()` permits it | the signal the feature is named for, so it is the unit the others are relative to |
| `acoustic` | `0.5` | `0.0` ignores loudness, brightness and noise floor entirely | half the semantic weight, so production differences can break a near-tie between candidates the words cannot tell apart without overriding what a person typed. **Not tuned:** it was first argued from a six-clip measurement the golden set superseded, and a person's judgement of fit (`#35`) is the evidence that would set it |
| `tempo` | `0.35` | `0.0` ignores rhythmic clash in the score, though the **hard rule still excludes** beyond the tolerance | below `acoustic` because a tempo estimate carries a stated ±2 BPM floor and is absent for every effect — a signal with a hole in it earns less weight than one without |
| `key` | `0.35` | `0.0` ignores key in the score | equal to `tempo` and for the same reason; equal rather than ranked because no measurement says which matters more |

**All four at `0.0` is refused** with `WeightProblem::AllZero` — nothing would rank. A **negative** weight is refused by
field name, because a signal that should attract would repel. A **non-finite** one is refused rather than compared false
against everything.

## The thresholds

| field | default | effect | why |
|---|---|---|---|
| `loudness_tolerance_db` | `6.0` | a pair further apart is **excluded** | a game mix moves in steps of a few dB — the crate's own default duck is 8 dB — so two sounds more than 6 dB apart are deliberately at different levels |
| `tempo_tolerance_bpm` | `4.0` | a pair further apart is **excluded** | **twice** the ±2 BPM accuracy floor ADR 0003 set, so a pair is only excluded when the difference is larger than both estimates' uncertainty. At 120 BPM that is ±3.3 % |
| `noise_floor_tolerance_db` | `12.0` | a pair further apart is **penalised**, never excluded | twice the loudness tolerance, because a hissier take is a worse neighbour rather than an impossible one. **Design default** — roughly where a noise floor stops reading as "the same room" |
| `compatibility` | see below | a key clash is **penalised**, never excluded | a modal piece sits anywhere, and key detection is the least reliable feature here |
| `drop_fraction` | `0.0` | the share of neighbours to drop, worst first | **keep everything**: at the 0.126 the correct band bottoms out at, recall@1 stays 100 %, but at 0.150 it falls to 75 % and at 0.5 to nothing. A filter that hides the right answer is worse than a long list |

## `compatibility`

```ron
compatibility: (clash_penalty: 0.5, parallel_penalty: 0.85)
```

| field | default | effect |
|---|---|---|
| `clash_penalty` | `0.5` | the multiplier for a semitone or a tritone — off the circle of fifths. **`1.0` stops the rule penalising**, so the number is the flag and no separate switch exists |
| `parallel_penalty` | `0.85` | the same tonic, opposite mode — C major against C minor, a deliberate shift rather than a clash |

A **fifth** is never penalised, and an **unknown key on either side** multiplies by `1.0`: an asset that was never judged
is never punished for it.

## The two weightings

The music table above ranks tracks. An effect has **no key and no tempo**, so `Weights::effects()` is a second table
rather than the first with numbers changed quietly:

| field | music | effects | why |
|---|---|---|---|
| `semantic` | 1.0 | 1.0 | the unit, in both |
| `acoustic` | 0.5 | **1.2** | an effect's cohesion is timbre, brightness, noise floor and loudness — it absorbs the weight the missing signals held |
| `tempo` | 0.35 | **0.0** | an effect has no tempo, and a zero says so honestly |
| `key` | 0.35 | **0.0** | the same |

The **totals match** (2.2), so a score from one table is comparable to the other's, and `shares()` never reports weight
spent on a signal that had no value. Both are **design defaults**: there is no labelled set of effects to measure against.

## Variants

```rust
Variants { identical_below: 0.99 }   // design default
```

Two results at or above this cosine are treated as **one sound**; the better-scoring one is returned and the other is
reported with the hash it resembled. High on purpose: the pitfall it exists for is a *list* of twelve near-duplicates,
not two sounds that resemble each other.

## Paths and versions

| item | value |
|---|---|
| model sample rate | `MODEL_SAMPLE_RATE = 48_000` |
| embedding width | `EMBEDDING_DIM = 512` (audio), `TEXT_EMBEDDING_DIM = 512` (text) |
| text token limit | `MAX_TEXT_TOKENS = 77` — a longer description is **refused**, never truncated mid-token |
| mel frames | `MODEL_MEL_FRAMES = 1001` |
| measured model version | `MEASURED_CLAP_VERSION = "transformers-5.22.1/torch-2.15.0/opset-17"` |

**There is no model path in the configuration**, and that is deliberate: `Model::load(path)` takes it as an argument, so
the crate never guesses where a 501 MB graph lives. A project pins one by loading that file; the index records the stamp,
and `Index::load` **refuses** an index built by a different stamp with both versions named. `Index::load_allowing(..,
true)` overrides it and records the mismatch on `Index::overridden`, so the decision is visible rather than swallowed.

## A worked example: a deliberately inconsistent palette

A stylised world where loudness jumps are intended, and where a clashing key is a design choice:

```ron
(
    semantic: 1.0,
    acoustic: 0.1,               // production matters little; the words do
    tempo: 0.35,
    key: 0.35,
    loudness_tolerance_db: 40.0, // the jumps are intended
    compatibility: (clash_penalty: 1.0, parallel_penalty: 1.0),
)
```

`problems()` returns **nothing** for this, and a test asserts that: the issue's own example is a legitimate
configuration, and the report must not punish it.

## See also

- [`semantic.md`](semantic.md) — the pipeline, the filter rules, and the ensemble's scale
- [`cues.md`](cues.md) — the cue table's own configuration
