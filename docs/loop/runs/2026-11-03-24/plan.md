# plan.md — iteration 24: the log keeps a refusal longer than a play

## The pick, and why it beats the runner-up

**Pick:** `SirenLog` evicts the oldest entry when full, and every entry is equally evictable. One `Refused` is pushed out
by **256 routine plays** — measured exactly — which in a 32-voice scene at 20 Hz is **~0.4 s**. That is the silent-failure
class the crate exists to prevent: the line explaining why nothing played is evicted by lines saying something did.

Beats the runner-up (nothing else is red this iteration: the log's cap arithmetic, `recent` past the buffer, `line()`'s
allocation, `next_boundary` at awkward tempos, and every no-ears spatial shape all came back clean): this one is a
**lost diagnostic**, and it is a change inside one private method.

## The design

In `SirenLog::push`, when the buffer is full:

1. drop the **oldest routine** entry — `Played`, `Stopped` or `Ducked`; else
2. drop the **oldest** entry (every entry is a refusal, which means the game is already failing loudly).

`Refused` is the exceptional class: a healthy game produces none, while a busy scene produces the other three at frame
rate. Nothing else changes — same 256 entries, same memory, same `recent`/`iter`/`len`.

## Fields

None. No `CueTable`, `SirenSettings` or `MusicScore` change; no public type changed; `CAPACITY` stays 256.

## Tests

- `a_refusal_outlives_a_burst_of_plays` — one refusal, then `CAPACITY` plays: the refusal is **still there**, and the
  buffer is still 256. **Fails on the base commit** (gone after 256).
- `the_buffer_never_exceeds_its_capacity` — the guard: `len() <= CAPACITY` after 1000 pushes, so the new eviction cannot
  grow the ring.
- `a_log_of_only_refusals_still_evicts` — the degenerate case: 300 refusals leave 256, because keeping them all is
  impossible and dropping the oldest is the only answer left.
- `entries_stay_in_order` — `iter()` remains oldest-first after an eviction from the middle-class rule, which is what a
  panel drawing a column depends on.

## The carve

- **Not** a separate refusal ring or a new accessor: a second list for one message, and a panel merging two streams by
  `at_s` — the crate's own trap about a second set of rules.
- **Not** `CAPACITY`: 256 is the panel's budget and `recent(8)` is what is drawn; the defect is *which* entry leaves.
