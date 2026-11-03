# Iteration 102 — review

## Pass 1 — function

**`#37`'s four criteria:**

| criterion | evidence |
|---|---|
| a load with insufficient free VRAM is refused naming the request and the availability | `a_short_device_is_refused_naming_both_numbers` — **623 MB** needed, **60 MB** free, and the sentence says *"Close what is using the GPU"* |
| the chosen policy is stated and observable | **defer, bounded** — `Deferral { wait_s: 30, poll_s: 0.5 }`, with `WaitState::Waiting { attempt, of }` the panel renders as *"waiting for the GPU (2 of 60)"*. `a_device_that_frees_is_caught_on_the_next_poll` and `a_device_that_stays_busy_is_refused_after_the_stated_bound` pin both ends |
| a query that needs no GPU still works while indexing is deferred | `a_query_still_answers_while_the_build_waits` — a real `top_k` runs **between two polls**, twice |
| the behaviour is documented for a user whose editor is rendering | `Deferral`'s doc states the three candidates, why two were rejected, and cites the source |

**The choice among `#37`'s three candidates, argued rather than picked:**

- **shrink the batch** → cannot work: `Budget::batch_capacity` already does it, and a batch of one still cannot load
  623 MB into 60 MB;
- **queue** → contradicts the source. Choi et al. say *"the applications cannot wait for the batch collection
  indefinitely"*, and an unbounded wait is a caller with no answer;
- **defer, bounded** → the only one that satisfies criterion 3, because it leaves the process alive and the index readable.

**Verdict: GREAT.**

## Pass 2 — zoom

Nothing audible changes, so the zoom is the **numbers in the sentences**, which is where this iteration's defects would
show.

- **A failure in my own test found a real arithmetic fact**: I asserted the message said `628 MB` and it said **623**. The
  figure is 116.5 MB of weights plus the 512 MiB overhead; my 628 came from rounding the two constants separately and
  adding them. The test now asserts the **computed** value and states the derivation, so the number in the message and the
  number in the test cannot drift apart.
- The **29.5 s** slept is `59 × 0.5`, one poll fewer than the 60 attempts — the last attempt reports `GaveUp` without
  sleeping past the bound. Asserted exactly rather than approximately, and asserted `<= wait_s`.
- **A 3.1× spread in the per-frame figures** across three runs of unchanged code (256 voices: 0.401 / 0.471 / 1.263).
  Recorded in `perf.md` as the machine's variance rather than as an improvement. **This is the honest finding of the
  iteration**: prior iterations reporting "unchanged" on this spread have been reporting luck.
- All **21 rendered wavs byte-identical** to iteration 101 (`cmp`).

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Read as a developer whose editor is rendering:

- `WaitState::describe()` returns `None` for `Ready`, so a panel shows nothing rather than *"ready"* on every frame — the
  state is a **transition**, not a status line.
- `Deferral::attempts()` derives from the two fields, so a user who halves `poll_s` doubles the looks without a second
  number to keep in step. Its degenerate cases return **1** rather than dividing by zero or looping forever.
- `defer` takes `measure`, `sleep` and `report` as the caller's, so the function **decides and performs no I/O**. That is
  why a test can prove a thirty-second bound in microseconds — and it is the same shape `Session::admit`'s callers
  already use.
- An expired deferral is a `BudgetProblem`, never `Ok`. The failure this policy must not have — a wait that quietly
  becomes a discard, leaving a half-built index and nothing saying so — is structurally impossible: every path returns
  either a session or a named refusal.

**One criticism, recorded:** `admit_deferring` takes seven arguments, and clippy's threshold is seven, so it just passes.
A `DeferralRequest` bundle would be kinder, and the same criticism was made of `search_ranked` before it was bundled. It is
a follow-up rather than a fix here, because bundling would change a signature nothing else calls yet.

**Verdict: GREAT.**
