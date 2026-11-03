# Iteration 97 — research: the workflow as calls, and the scale a penalty has to live on

## The item

`#42` (`set_anchor`, `find_compatible_music`), `#43` (`find_compatible_sfx`), `#44` (`explain_match`). All three were
blocked on `#40`, which iteration 96 wired.

## Sources

| Source | Number it reports |
|---|---|
| `#43`'s own text | *"an effect library often has hundreds of near-identical variants (footstep 01–12). The result must not return twelve variants of one footstep as the top twelve."* |
| `#42`'s own text | *"two clients setting different anchors on one server will interleave. Either refuse concurrent clients or key the anchor per session — decide and document it."* |
| `Weights` (this crate) | loudness 6 dB, tempo 4 BPM, noise floor 12 dB — the tolerances both the rules and the scale read |
| `filter::exclude` (this crate) | loudness and tempo **exclude**; noise floor and key **penalise** — the distinction a scale must preserve |
| `~/.cargo/registry/src/index.crates.io-*/rodio` — not used here | — |

## The technique

**An anchor is state, and its scope is the object that holds it.** `Session` is one optional hash. Two clients each
hold their own, so `#42`'s interleaving cannot occur, and a restart loses the anchor because a restart should.

**Two weightings, not one changed quietly.** `Weights::effects()` moves the 0.35 + 0.35 that key and tempo held onto
acoustic agreement (1.2), keeping the total at 2.2 so a score from one weighting is comparable to the other's.

**Variants are suppressed after ordering**, so the survivor is the best-scoring of a near-duplicate group rather than an
arbitrary file; and the omission is **reported** with the hash it resembled, because a caller choosing between three
footsteps still wants to know nine more exist.

## The finding: a `[0,1]` scale cannot explain a result

The first implementation mapped every signal into `[0, 1]`. It compiled, and 12 of 15 tests passed. The three that
failed showed why the scale is wrong:

- `Breakdown::strongest()` reports the strongest **positive** and the strongest **negative** contribution. On `[0, 1]`
  no contribution can be negative, so `explain()` could *never* name what counted against a result — a breakdown that
  is really a score with decoration.
- A cosine of `−0.2` (two unrelated sounds) would have scored `0.4` of agreement.

The scale is therefore **signed, `[-1, 1]`**: semantic is the bare cosine (already on it), acoustic and tempo are
`1 − 2·difference/tolerance` (so the tolerance, the point the rule calls fatal, maps to `−1`), and key is
`2·multiplier − 1` (so the table's `1.0` is `+1` and its `0.5` is neutral).

## Parameters

| Parameter | Value | Source |
|---|---|---|
| `Variants::identical_below` | 0.99 | **design default** — high on purpose: `#43`'s pitfall is a *list* of near-duplicates, not two sounds that resemble each other |
| `Weights::effects().semantic` | 1.0 | **design default** — the unit, as in the music table |
| `Weights::effects().acoustic` | 1.2 | **design default** — absorbs the 0.35 + 0.35 the missing signals held; total preserved at 2.2 |
| `Weights::effects().tempo`, `.key` | 0.0 | **design default** — an effect has neither, and a zero says so honestly |

## What good looks like

A result that explains itself in one sentence naming both directions, a list that does not repeat one footstep twelve
times but says it left nine out, and a refusal with a second anchor that returns the *new* anchor's neighbours.
