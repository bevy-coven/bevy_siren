# plan.md — iteration 25: the capture harness runs with no environment at all

## The pick, and why it beats the runner-up

**Pick:** `examples/mixer` — the crate's **only** visible harness, and the one the skill names for judging a visible
change — **panics** on the command its own module doc gives, unless `SIREN_KEYS` is set. The doc lists that variable
beside three that default. `% 0` at `mixer.rs:376`.

Beats the runner-up (the four `CueProblem` variants): those are **blocked** on a public-enum decision the owner has not
answered, and iteration 24's log asked the question rather than picking again. This one is unblocked, is a crash, and
restores the loop's own evidence path.

## The design

Two changes, both in `examples/mixer.rs::capture_config`:

1. **`SIREN_KEYS` unset ⇒ `steal,layer,duck`**, the script the module doc describes.
2. **A list that parses to nothing prints one line and uses the default**, so a typo cannot silently produce a capture
   of something else under an evidence name.

`stage_keys` already guards the index with `let Some(step) = c.keys.get(…) else { return }`; the panic was the **modulus**,
not the lookup, and with (1) and (2) the divisor is never zero.

## Fields

None. Example only — no `src/` change, no public type, no serde schema.

## Tests

The example is a binary, not a library, so it has no unit tests in this repo's harness. What proves it:

- **Run it with no environment at all**: `cargo run --release --example mixer` must not panic (it is interactive, so the
  capture is run with only `SIREN_FRAMES`, which is the default path this fix repairs).
- **A still at both sizes**, 1280×720 and 1920×1080, **looked at** — the skill's step 6 — in
  `evidence/panel/{mixer-1280,mixer-1920,zoom-buses,zoom-log}.png`.
- **`SIREN_KEYS=bogus`** prints the notice and still writes its still.
- **`SIREN_KEYS=layer`** stages only layers, so the explicit path is unchanged.

## The carve

- **Not** a `#[test]` for the example: this repo's test harness links `tests/all.rs` against the library, and an example's
  `env`-driven config is not reachable from it. The proof is the run, recorded.
- **Not** a panic on a bad token: the example's job is to produce evidence on an unattended machine; the bug was the
  silence, and the silence is gone.
