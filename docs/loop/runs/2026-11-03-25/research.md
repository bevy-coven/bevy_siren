# research.md — iteration 25: the capture harness panics on the command its own doc gives

## The item

`examples/mixer` is the crate's **visible** harness — the skill names it as the place to capture a change, and the module
doc's first line is `cargo run --release --example mixer`. Its capture knobs are documented as **optional**:
`SIREN_FRAMES=<n>` saves a still, `SIREN_KEYS=steal,layer,duck` stages steps. Run with `SIREN_FRAMES` and no
`SIREN_KEYS` — the default — and the process panics:

```
thread 'Compute Task Pool (0)' panicked at examples/mixer.rs:376:33:
attempt to calculate the remainder with a divisor of zero
```

`capture_config` built `keys` from `std::env::var("SIREN_KEYS").unwrap_or_default()`. Unset gives `""`; `"".split(',')`
gives one empty token; no branch matches it; `keys` is **empty**; `stage_keys` then evaluates
`((since / every) as usize) % c.keys.len()`, which is `% 0`.

So the harness that exists to produce evidence **cannot produce any** unless the operator happens to know the one
variable its doc calls optional.

## Sources

**S1. The module doc's own promise** — *"`SIREN_KEYS=steal,layer,duck` stages one step every `SIREN_KEY_EVERY` frames"*,
listed beside `SIREN_FRAMES`, `SIREN_SHOT_DIR` and `SIREN_SIZE`, all of which default. The doc's framing is that the
knobs tune a capture that works.

**S2. The crate's **fail loud** rule** — *"a bad cue is refused with a reason, never played wrong"*, and the skill's own
step 6: *"Nothing ships without … panel screenshots you looked at."* A harness that panics cannot be looked at, so every
visible change this loop ships has been evidenced by the headless `examples/render` alone.

**S3. `capture_config`'s shape** — every other knob has a default: `at` 120, `shot` `mixer.png`, `from` 30, `every` 2,
`key_every` 20. `keys` alone had none, and it is the one whose absence is **fatal** rather than cosmetic.

**S4. Rust's own arithmetic** — `% 0` on an integer is a panic in every profile, not a wrapped value: this is not a
debug-only assertion, it is the release path the doc names.

## The technique

Two changes in `capture_config`, both inside the example:

1. **The default script is the documented one.** `SIREN_KEYS` unset now yields `steal,layer,duck` — the three things the
   module doc says the panel is for, *"the last one before the still is the duck, which is still holding when the
   screenshot is taken"*.
2. **A list that parses to nothing says so.** `SIREN_KEYS=bogus` is a typo, and a capture that silently staged a
   different script would put the **wrong picture** in an evidence folder. It now prints
   `siren: SIREN_KEYS has no step this example knows; staging the default steal,layer,duck` and continues.

**Why not a panic on an unrecognised token?** Because the example's job is to produce a picture on a machine that may be
a CI box with no operator; a loud line plus the documented default is a better failure than no evidence at all. The
**silence** was the bug, not the fallback.

**Belt and braces:** `stage_keys` keeps its `let Some(step) = … else { return }`, so even a future refactor that leaves
`keys` empty cannot panic — the modulo result is only used to **index**, and `get` returns `None`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the three staged steps, in order | `steal,layer,duck` | S1 |
| the default still frame | 120 | S3 |
| every other knob has a default | — | S3 |
| `% 0` panics in release too | — | S4 |

## What good sounds like

`cargo run --release --example mixer` with **no** environment variables draws the panel for a moment and exits; with
`SIREN_FRAMES=140` it writes a still that shows a duck holding on the Music bus, a voice that was stolen, and the log's
own account of both. Nothing about any sound changes.
