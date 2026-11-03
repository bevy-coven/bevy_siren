# The human listening check

No metric in this crate measures whether a result **sounds** right in a game. The automated gates judge spectrograms and
loudness because their author cannot hear the result; this page is the step that needs a person, written down so it is
done rather than assumed.

Short and repeatable beats long and skipped. This is under ten minutes.

## Before you start

You need:

- a library of **at least six** real assets you know, in one folder;
- an index of it (`indexbench` builds one, and so does a normal build);
- a description that names **one of those assets**, which is the anchor.

## The commands

```sh
# 1. Put the anchor and its neighbours in rank order, numbered, with their scores beside them.
cargo run --release --features semantic --example audition -- <library-dir> 6

# 2. Listen. Files land in docs/loop/runs/<run>/evidence/audition/, named 01-first.wav … 06-last.wav.
#    Play them in order. DO NOT open README.txt yet.

# 3. Then read what the ensemble said.
cat docs/loop/runs/<run>/evidence/audition/README.txt

# 4. Write the outcome into the run's review.md, using the shape below.
```

## What to listen for, in this order

The order matters more than the list: a later question is unanswerable if an earlier one already failed.

1. **Does `01` belong beside the anchor?** Not *"is it good"* — *"would these two sit in the same scene"*. This is the one
   question the whole feature exists to answer.
2. **Is `01` better than `02`?** If not, the ranking is inverted, which is worse than a weak top result because it means
   the score is measuring something other than fit.
3. **Is the worst of the six still plausible?** A long tail of unrelated assets means the floor is too low; a tail that is
   *right but dull* is fine.
4. **Does any one asset repeat?** Twelve takes of one footstep filling the list is `Variants` failing.
5. **Only now, compare with the README.** For each file, did the signal the ensemble says **carried** it match what you
   actually heard? A result carried by `acoustic` that sounds like a timbre match is the ensemble weighing production
   where it should weigh timbre — that is a finding, and it goes in the record rather than a footnote.

## What counts as a failure

| observation | what it means | follow-up |
|---|---|---|
| `01` does not belong | the ranking is wrong at its top | a named issue with the anchor, the query and `01`'s hash |
| `02` is clearly better than `01` | the score is inverted | a named issue with both breakdowns attached |
| every result is unrelated | the floor or the weights are wrong | a named issue naming which |
| one asset appears three times | `Variants::identical_below` is too low | a named issue with the pair's cosine |
| the README's `carried` signal contradicts what you heard | the scale or a weight is wrong | a named issue quoting `Breakdown::explain()` |

**Any failure gets its own issue.** A failure recorded only in a run's `review.md` is a failure nobody fixes.

## Recording the outcome

In the run's `review.md`:

```md
## The listening check — <date>, by <who>

Anchor: <hash> (<file>). k=6.

1. Does 01 belong beside the anchor? <yes/no, and what it sounded like>
2. Is 01 better than 02? <yes/no>
3. Is the worst plausible? <yes/no>
4. Does any asset repeat? <yes/no>
5. Did the `carried` signal match what I heard? <yes/no/which disagreed>

Failures filed: #<n>, #<n> — or "none".
```

**The person who listened signs it.** An unattributed result is not a result, and a check whose outcome is anonymous
cannot be re-run when the weights change.

## The first run

Recorded in [`docs/loop/runs/2026-11-03-99/review.md`](../loop/runs/2026-11-03-99/review.md): the **script and procedure
were exercised** — three assets, rank order, `README.txt` written — but the **listening step is unperformed**, because
the machine that built this has no listener. That is stated there rather than filled in, and it is the one acceptance
criterion of `#46` this repository cannot close on its own.
