# research.md — iteration 11: the half of "never stolen" that had no test

## What this iteration is

Iteration 10's next-5 led with `DuckClock.started`'s staleness; **that item is not a bug**, and this run says so with a
measurement rather than a fix. `started` is keyed by **rule**, `(source, target)`, not by fire: 50 fires over the two
default rules leave exactly two entries. A pair whose buses have left the table is inert, because `duck_db` reads
`started` only through `settings.ducks` — the map cannot grow past the rule count, and a stale instant is never
consulted.

With that closed, five further probes found no rank-1 defect: a stop on the play's own frame still fades (the gain is
written before `stop_requests` in the same set); `Played.gain_db` and the audible chain agree to the hundredth
(−2 cue −3 bus −6 master = −11.00 dB, `Applied` −11.0); the unstealable promise holds against both a third unstealable
request and a stealable one; `bus_voice_limit` clamps a bus's cap to the table's; `SirenLog`'s ring keeps 256 and drops
the oldest first.

That leaves the crate's strongest promise — `UNSTEALABLE_PRIORITY` is *"never stolen"* — as the one place worth adding a
**clearer** statement of the rule. My first reading was that the promise had an untested half: the existing table-budget
test asks the *same* unstealable cue for a second voice, which looked like it proved only "unstealable refuses
unstealable". **That reading was wrong, and this iteration's own mutation check proved it**: removing `steal`'s
`!c.unstealable` filter reddens `the_table_budget_refuses_rather_than_grow`, because the second request then steals its
own older voice instead of being refused. The suite **did** cover the promise; it covered it through a path a reader has
to reason about rather than through the case the rule names.

So this iteration ships no repair. What it ships is a **direct** statement of the rule — a priority-0 request against a
priority-200 voice — and the mutation measurement that shows both the old and the new tests catch the same break. The
value is legibility, not a closed hole, and the log says exactly that.

## Sources

1. **Gregory, *Game Engine Architecture* 3e, §14.5.8.4 "Prioritization and Voice Stealing"** (this crate's citation for
   R4): *"some cues might be temporarily marked as 'unstealable' to ensure that they play, even despite their priority
   settings."* **Even despite their priority settings** — the marking overrides priority in both directions: it does not
   merely keep a high-priority voice from being stolen by an equal one, it keeps it from being stolen at all.

2. **The crate's rule R4** (`docs/research/siren.md`): *"never a voice at `UNSTEALABLE_PRIORITY` (music, stingers,
   dialogue)"*. The rule is about the **voice**, not about the request that arrives; a request whose own priority is 0
   is still not allowed to take one.

3. **`model::steal`**, the pure function both caps use: it filters `!c.unstealable` before sorting, and returns an empty
   answer when too few stealable candidates exist. `tests/model.rs::steal_takes_low_priority_then_the_oldest` pins that
   at the arithmetic level, with one unstealable candidate among four.

4. **`tests/mix.rs::the_table_budget_refuses_rather_than_grow`** exercises the **system** path, and its second request is
   the *same* unstealable cue. The rule it proves is "a full budget of unstealable voices refuses another", and — checked
   by mutation rather than assumed — it **does** fail when `steal` ignores `unstealable`, because the second request then
   takes its own older voice. The suite covered the promise; the test's path is one a reader has to reason about.

5. **`cues.yaml` case 9**: *"A voice a player is listening to is never stolen; the request is refused instead …
   `Refusal::VoiceLimit`, nothing spawned, the dialogue voice still live."* The case's words name the **voice**, and no
   test asked a request of *different* priority whether it could take one — which is why a direct one is worth having even
   though the path is covered.

**Literature server unavailable, ninth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is Gregory's sentence, which this repository already holds.

## The technique

Nothing in `src/` changes. The work is a test that asks the question nobody asked:

> With every voice in the table unstealable, does a **stealable** request play?
>
> It must not. It must be `Refused { VoiceLimit }`, nothing spawned, and **no `ended`/`stolen by` line** — a steal it
> cannot make may not be logged as one.

A second test pins the other side so the pair cannot both pass by refusing everything: with one unstealable voice and
room to spare, a **stealable** request plays, and the unstealable voice is untouched.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what `UNSTEALABLE_PRIORITY` means | never taken, **whatever the arriving request's priority** | Gregory §14.5.8.4; crate R4 |
| the value | 180 | crate design default, unchanged |
| the refusal | `VoiceLimit`, nothing spawned | R4, unchanged |
| the log | no `stolen by` line for a steal that did not happen | R6 — two records of one request cannot disagree |

## What good sounds like

Nothing. No sample, gain or duck moves; no line in `src/` changes. What changes is how directly a reader can see the
rule: `an_unstealable_voice_is_never_taken_by_a_stealable_request` says in its name what Gregory's sentence says, and a
priority-0 request is the sharpest way to ask it.

A reviewer judges it by the mutation: with `steal`'s `!c.unstealable` filter removed, **three** tests go red — the two new
ones and `the_table_budget_refuses_rather_than_grow` — which is the honest measurement. The new tests make the rule
legible; they do not close a hole, because there was none.
