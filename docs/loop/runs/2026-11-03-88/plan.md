# Iteration 88 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#36` indexing throughput** | Owed three iterations, and three costs were added to `build` in 85–86 with nothing to say whether they are affordable. It is the last unmeasured number in the layer. |
| 2 | `#22` spectral features | Unblocked, larger. |
| 3 | `#28` the MCP tool host | Largest user-facing win. |
| 4 | `#27` hard filters | The consumer of everything since 77. |
| 5 | `f16`, with `V-01` | Needs a labelled set. |

**Why (1) beats (2) in one sentence:** #22 adds a feature whose cost is unknown anyway, while #36 is the measurement
that says whether the last three iterations' additions were affordable at all.

## The change

**`examples/indexbench`**, registered with `required-features = ["semantic"]`, measuring the pipeline in two stages.

| Decision | Choice | Why |
|---|---|---|
| separate example | yes, not a `perfcheck` mode | #36's own note: the two numbers must never be confused |
| two stages | decode, then decode + synthetic embed | one number cannot name the bound |
| corpus | generated, 0.5 s tones | reproducible on any machine, no licensed library needed |
| corpus reuse | an existing non-empty directory is used as given | a real library can be measured by pointing at it |
| the synthetic constant | `512 × 512` multiply-adds | **design default**, named in the output |

## Modules touched

| File | Change |
|---|---|
| `examples/indexbench.rs` | **new** |
| `Cargo.toml` | one `[[example]]` entry |

No `src/` change, no new dependency, no new threshold.

## The numbers

| clips | decode | total |
|---|---|---|
| 200 | **2 411 /s** | **1 856 /s** |
| 500 | **2 636 /s** | **1 812 /s** |
| 1000 | **2 724 /s** | **1 846 /s** |

**5 000 clips: 3 s** at the measured rate, excluding real inference.

## Carved off, and why — the GPU half

`#36`'s criteria also ask for **peak VRAM** and the bound on the **RTX 3090**. Measured this iteration: `big` is
reachable (RTX 3090, 24 576 MiB, 24 cores) and **holds a checkout at `8e14231`**, which predates every semantic change
since iteration 62 — there is no `build_from_model`, no `loudness`, no sidecar, and **no ONNX graph on the host**.

Producing a GPU number needs: the current commit cloned there, the 116.5 MB audio graph exported and transferred, and
`ort` built against CUDA. That is a session of its own, and reporting a stale-checkout number would be worse than
reporting none — which is what the issue's own *"the target must come from a measurement on the real target hardware,
not from an aspiration"* warns against.

**So the GPU half stays open**, `#36` stays open, and the decode-bound half — which is the half that does **not** need
the accelerator — is measured and shipped.
