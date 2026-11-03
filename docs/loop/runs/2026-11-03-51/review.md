# review.md — iteration 51, three passes

## Pass 1 — Function: does `avoid` mean what `Repeat::AvoidRepeat` says?

**The claim** (`src/cue.rs:186`): *"Random, but never one of the last `avoid` variants — the default, and what kills the
machine-gun footstep."*

**Measured**, four keys, seed 11, 24 picks, counting picks between a key and its previous appearance:

| | picks returning within 3 | gaps |
|---|---|---|
| before | **16 of 20** | `2 2 6 3 3 3 3 2 2 3 3 4 2 4 2 2 2` |
| after | **0 of 20** | every gap exactly **4** |

`avoid: 3` asks for a gap of at least 4, and after the fix that is what the stream is. The plot is
`evidence/plots/variant-window.svg`; the streams are in `evidence/after/render.txt` as `variants <cue>:` lines, which
this iteration added to the render harness so the rule is written down as a number rather than inferred from a wav.

**The regression test fails on the old code.** `tests/play.rs::a_cue_whose_avoid_grows_gets_the_wider_window`, stashed
`src/` and the new model tests to compile it against the old tree:

```
at `avoid: 3` a key must not return within three picks:
[Some("d"), Some("c"), Some("b"), Some("a"), Some("c"), Some("d"), ...]
```

`d` returns after two picks. On the fixed tree the same test passes.

**Verdict: GREAT.**

## Pass 2 — Zoom: the mechanism, and the second bug inside it

**The window was the allocator's, not the author's.** `remember` did
`let capacity = self.history.capacity();` — `VecDeque::with_capacity(n)` may reserve **more** than `n`, and it never
grows on demand from a picker built small. So the window was two different numbers at once: the one the caller passed
to `new`, and whatever the allocator rounded it to. Replaced by an explicit `window: usize` field, set by `new` and
`set_capacity`, and read by `remember`/`remember_key`.

**A `derive(Default)` picker remembered nothing.** With the explicit field, `Default` would give `window: 0`, and
`remember` returns early on zero — so a default picker silently refuses nothing. Written by hand as `new(1)`, matching
`Repeat::default()`. `a_default_picker_remembers_one_pick` asserts it.

**`reset_histories` has no caller** in this crate, its tests, or the Cauldron. That is a consumer item (the panel's table
reload should call it) and is logged, not patched here.

**The test that proved it needed fixing twice.** My first version asserted through `pick_from` alone, which still failed
after the source change because growth happened only in `SirenChoices::pick`. Rather than teach callers about
`set_capacity`, the picker now grows itself inside `pick_from` — the rule travels with the call that reads it.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: a different seed, and a table whose window cannot be met

`examples/render`'s own table contains the case the crate cannot honour: `footstep.stone` has **two sounds and
`avoid: 4`**. Its stream (`evidence/after/render.txt`) is `stone_02 stone_01 stone_02 …` — a perfect alternation, the
best a two-variant cue can do, and identical before and after, which is the right answer: no window of four exists in
two keys.

What is **not** right is that nothing says so. `pick_from`'s own comment promises *"`problems()` is where the table
should be named"*, and probing `CueTable::problems()` for a two-sound cue at `avoid: 5` returns **`problems: []`**. That
needs a `CueProblem` variant, which needs the open `bad-gain-problem.md` enum decision — so it is logged, not built.

**Can a developer read it without the code?** Yes: the `variants` lines state the rule, the sound count and the longest
run, so a machine-gun footstep is visible as `longest run 1` against `avoid 1`, and a thin cue as `avoid 4, 2 sounds`.

**Verdict: GREAT.**
