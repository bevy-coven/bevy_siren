# Iteration 102 — research: what a busy GPU should do to an indexing run

## The item

`#37`. The RTX 3090 is shared with a rendering editor. An indexing run during a render either slows the editor or dies
on an out-of-memory, and *neither should be discovered by a user*.

## Sources

| Source | What it reports |
|---|---|
| Choi, Lee, Kim, Park, Kwon & Huh, *Multi-model Machine Learning Inference Serving with GPU Spatial Partitioning* — **DOI 10.48550/arXiv.2109.01611** | the two constraints a shared accelerator imposes: *"they must provide a **bounded latency** for each request … and they can serve multiple heterogeneous ML models"*. Batching to maximise throughput is bounded by an **SLO**, not by a heuristic: *"the applications cannot wait for the batch collection indefinitely"*. The mechanism is a **`gpu-let`** — a reserved share rather than a free-for-all |
| This crate's own `Budget` (`src/semantic/budget.rs`) | refuses with `DoesNotFit { need_bytes, available_bytes }` — **two numbers**, so the reader knows whether to close something or buy a card |
| `docs/loop/cues.yaml` case 33 | the crate's existing answer to a scarce resource: Gregory §14.5.8's **virtual voice** — *"a sound the game believes is playing but that has no hardware voice"* — retried when room frees |
| Gregory, *Game Engine Architecture* 3e §14.5.8 | a fixed resource that **refuses politely and retries**, rather than failing inside a library |

## The technique

`#37` asks for **one** policy out of *queue, shrink the batch, or defer until the device is free*, with the choice
stated. The crate's own precedent decides it:

- **Shrinking the batch** is already `Budget::batch_capacity` and `session::embed_batched`'s shrink — and it does not
  answer this issue, because a batch of 1 still cannot load a 116 MB model into a card with 60 MB free.
- **Queueing indefinitely** contradicts the source's own point: *"the applications cannot wait for the batch collection
  indefinitely"*. A server that holds a request forever is one whose caller has no answer.
- **Deferring, with a bounded wait and an observable state**, is what the virtual-voice precedent already does and what
  a panel can actually show. It is also the only one that satisfies `#37`'s own criterion *"a query that needs no GPU
  still works while indexing is deferred"*: deferral leaves the process alive and the index readable.

**The choice: defer, with a stated bound and a state.** The bound exists because the source says latency must be
bounded; the state exists because `#37` asks for *"a user-visible state so a panel can show 'waiting for the GPU'"*.

## Parameters

| parameter | value | source |
|---|---|---|
| `Deferral::wait_s` | **30 s** | **design default** — long enough for a render frame's transient peak to clear, short enough that a user is not left staring; the source's SLO argument is why it is bounded at all |
| `Deferral::poll_s` | **0.5 s** | **design default** — re-checking twice a second bounds the worst-case latency added to a deferral by half a second |
| retry ceiling | **no separate field** — `wait_s / poll_s` attempts | derived, so two numbers cannot disagree |

## What good sounds like

A panel reads *"waiting for the GPU (1 of 60)"* rather than hanging. A query on an existing index answers in
milliseconds throughout. When the render ends, the index build proceeds without the user retyping anything — and if it
never ends, the refusal names both numbers and says what to close.

## The failure this must not have

**A deferral that silently becomes a discard.** If the wait expires and the caller is not told, the index is
half-built and nothing says so — exactly the silence `#37` exists to remove. So an expired deferral is a **named
refusal** carrying the request and the availability, never an empty success.
