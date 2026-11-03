# review.md — iteration 11, three passes

## Pass 1 — function

**What this iteration claims.** Nothing is repaired. Two tests state `UNSTEALABLE_PRIORITY`'s rule **directly** — a
priority-0 request against a priority-200 voice — and a measurement retires `DuckClock.started` from the backlog.

**The honest part.** My first premise was that the promise had an untested half. **The mutation disproved it:**

| mutation | result |
|---|---|
| `steal`'s `!c.unstealable` filter removed | **three** tests red — both new ones **and** `the_table_budget_refuses_rather_than_grow` |

So the suite already caught the break, through the table-budget test's second request stealing its own older voice.
The new tests are a clearer statement of the rule, not a closed hole. `research.md` and `plan.md` are corrected to say
so; the earlier drafts claimed a gap that does not exist.

**Items retired by measurement, not by change:**

| candidate | measurement |
|---|---|
| `DuckClock.started` grows with table swaps | 50 fires over two rules leave **two** entries — keyed by rule, not by fire; `duck_db` reads only pairs still in `settings.ducks` |
| a stop on the play's own frame cuts instead of fading | `Applied` is written before `stop_requests` in the same set; the voice fades for 0.48 s |
| `Played.gain_db` disagrees with the audible chain | −2 cue −3 bus −6 master = **−11.00 dB**, and `Applied` reads **−11.0** |
| a stealable request takes an unstealable voice | refused, nothing stopped |
| a bus cap above the table's raises the budget | `bus_voice_limit` clamps with `.min(total)` |

**Verdict: GREAT** for what it is — a legibility change plus five negative results, reported as such.

## Pass 2 — zoom

**Read:** the mutation run's output, `evidence/{before,after}/render.txt`, the cue spectrograms and loudness.

**Audio is byte-identical**, which is required: no line of `src/` changed. 16/16 WAVs, `ui-cancel` −24.4 LUFS / −12.1 dBFS
looped past the gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS.

**One process defect in my own work:** the `before` tree was produced by a `git stash` that did not rebuild, so its WAVs
are the after-tree's binary — which is *correct* for a test-only iteration and is stated rather than presented as an
independent measurement. The spectrograms are therefore after-only, and there is nothing for a before to differ from.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The two new tests are a pair: one asserts the refusal, the other that a stealable voice
still plays beside an unstealable one, so neither can pass by the crate refusing everything — the rule `cues.yaml` states
for cases 10–13.

**Can a developer understand it without the code?** `an_unstealable_voice_is_never_taken_by_a_stealable_request` says in
its name what Gregory's sentence says. `cues.yaml` case 9 now names the stealable-request direction as its observation,
where before it named only the voice.

**What a fresh reader would still ask:** *is this worth an iteration?* The answer in the log is no, not as a repair — and
yes, as the last unblocked item: ten iterations have closed every rank-1 defect the suite and the probes could find, and
the remaining five items are public-enum decisions for the owner. A loop that manufactures a fix to look busy would be
the worse outcome.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 1 (a premise), and it was corrected by measurement.
