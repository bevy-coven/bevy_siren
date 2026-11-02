# Does `Completes` stay in the public API?

## The question

`bevy_siren::Completes` is a public component whose only state is a private `done: bool` that **nothing in the crate
can set**: `Completes::default()` is `false`, there is no constructor or setter, and `finished()` therefore always
returns `false`. It was added for the crate's own mixer to record "this sample ended", and iteration 2 found that the
mixer instead answers through `AudioOutput::finished` — the query it already implements — because `retire_voices`
has to ask *before* it despawns the entity. Does it stay?

## The options

1. **Remove it.** `pub struct Completes` and its re-export go. A consumer that queries `Completes` (nobody should —
   it can only ever be false) fails to compile, which is a loud break in a `0.x` crate. Cost: one public type, a
   `CHANGELOG` "Removed" line, and a version bump that a `0.1 → 0.2` already allows. Benefit: the API stops offering a
   type whose one method cannot be true, which is the sort of trap a host reads as "the crate will tell me when a
   voice ended" and then waits for a message that never comes.
2. **Keep it and set it.** A system asks `AudioOutput::finished` for every mixer voice each frame and writes
   `Completes { done: true }`; `retire_voices` then reads the component as it does today. Cost: a second reader of
   the output per frame and one frame of latency between "the sample ended" and "the voice is retired", so a budget
   of 32 holds 33 for one frame in the worst case — the opposite of the hard cap this crate promises. Benefit: the
   type is honest and a game can observe an end as a component change.
3. **Keep it, document it as reserved.** Cost: a public type that does nothing is still a promise the crate does not
   keep; `#![warn(missing_docs)]` is satisfied by a sentence that says "always false", which no host reads.
4. **Keep it as `#[doc(hidden)]` and stop re-exporting it.** Cost: a semver-visible removal of the re-export anyway,
   plus a type that still compiles inside the crate. Benefit: none over option 1.

## The recommendation

**Remove it (option 1), in the release that next bumps the minor version.** The crate's own rule is that a voice's
end is one decision made in one place — `retire_voices` — and a component that both records and cannot record that
decision is dead weight in the public API. Nothing in `docs/`, the guides, the examples or the Cauldron's panel
names `Completes`; a `rg` over the repository finds it only in `src/` and this decision. Option 2 costs the hard cap
its hardness, which is the one thing the budget exists for.

If the owner prefers no break at all, option 2 is the fallback, and its one-frame overshoot must then be written into
`docs/guide/buses.md` beside the budget's promise.

## What is blocked

Nothing in iteration 2: the fix reads `AudioOutput::finished` directly in `retire_voices`, so it works with or without
`Completes`. The type is left in place, unchanged, until this is answered. The loop did not delete it, because a
removal is a public-API change and this file is where the loop puts those.
