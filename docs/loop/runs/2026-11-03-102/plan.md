# Iteration 102 — plan: defer, bounded, with a state

## Ranked

1. **`#37`** — chosen. Blockers `#17` closed; needs no new dependency, no GPU and no editor, and it is the only open
   item that is **implementable and testable in this repository**. Its criterion 1 already exists (`Budget::fits`); what
   is missing is the policy, the state and the no-GPU guarantee.
2. `#28` (MCP server) — needs `serde_json`, absent from the graph (verified: `cargo tree` finds it **0** times), and
   `tests/leaf.rs` closes the list. Decision filed; not buildable unilaterally.
3. `#35`, `#32`–`#34`, `#38` — a person's labels, or the editor repo.
4. `#39` — a music encoder and a licence audit.
5. — nothing else is unblocked.

**Why 1 beats 2:** `#37` is implementable here today. `#28` needs a dependency decision only the owner can make, and a
server built on an undecided codec would be thrown away.

## Design

`src/semantic/budget.rs` gains:

- `Deferral { wait_s: 30.0, poll_s: 0.5 }` — a serde type, both defaults argued in `research.md`.
- `Deferral::attempts()` — derived from the two, so they cannot disagree.
- `WaitState` — `Ready` / `Waiting { attempt, of }` / `GaveUp`, the value a panel renders.
- `defer(budget, cost, deferral, mut probe) -> Result<u64, BudgetProblem>` — the policy. It calls the caller's
  `probe` (which reads the driver) up to `attempts()` times, sleeping `poll_s` between, and returns the attempt that
  succeeded or a **named refusal carrying the last measurement**.

The sleep is injectable so the test runs instantly and the policy is measured rather than slept through.

## Fields

| field | type | default | citation |
|---|---|---|---|
| `wait_s` | f32 | 30.0 | **design default**, bounded because the source requires bounded latency |
| `poll_s` | f32 | 0.5 | **design default**, bounds the added latency |

## Tests

- a short device is refused **naming both numbers** (criterion 1, already true, re-asserted at the policy level);
- a device that frees after N polls **succeeds on attempt N+1**, and the state was `Waiting` before it;
- a device that never frees **refuses after exactly `attempts()`** — not forever, not once (criterion 2);
- **a query works while the build defers** (criterion 3) — an index search returns throughout;
- the sleep is not taken in tests, and the attempt count is exact.
