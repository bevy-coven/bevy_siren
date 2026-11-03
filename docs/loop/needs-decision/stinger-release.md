# Does `Stinger` get its own release time?

## The question

A stinger ducks the `Music` bus by `duck_db` for `duck_s`, then lets it recover. `duck_s` now holds correctly
(iteration 3), but the **release** after the hold is fixed at the crate's `DEFAULT_FADE_S` (0.12 s). Should `Stinger`
carry a `release_s` field so a project can choose it?

## The options

1. **Add `release_s: f32` to `Stinger`, default `DEFAULT_FADE_S` (0.12 s).** Cost: a public message type gains a field.
   `Stinger { cue, duck_db, duck_s }` written as a struct literal — the form the crate's own examples use — stops
   compiling, so a consumer must add `..Stinger::new(..)` or name the new field. `Stinger::new` and the builder methods
   keep working. Benefit: the three numbers of a duck (`depth`, `hold`, `release`) are all a project's, matching `Duck`
   on the mix table, which has `by_db`, `hold_s` and `fade_s` already. A stinger is the only duck in the crate whose
   release a project cannot name.
2. **Keep it fixed at `DEFAULT_FADE_S` and document it.** Cost: a project that wants a long, cinematic release on a
   death stinger cannot have one; it must reach for `DuckClock::manual` directly, which is a second authoring path for
   the same idea. Benefit: no API change, and 0.12 s is the crate's own answer to "short enough to read as immediate".
3. **Read the release from the cue's own `Duck` rules** — that is, let a stinger use whatever `SirenSettings::ducks`
   says lowers `Music`. Cost: a stinger's shape becomes a property of the mix table rather than of the event, and a
   project with no duck rule on `Music` gets a stinger that does not duck at all. Benefit: one place to author it.
   This is a bigger semantic change than it looks and the loop does not recommend it.
4. **Add the field and also a `release_s(seconds)` builder**, defaulting as option 1. Cost: the same break, plus one more
   method. Benefit: consistent with `duck_db`/`duck_s`, which both have one.

## The recommendation

**Option 1 (or 4, which is 1 plus its builder), in the next minor version.** The mix table already gives a duck three
numbers; a stinger giving two and hard-coding the third is the inconsistency, and the loop's research says the release
is the part a project most often wants to be long (a stinger's tail outliving the hold). `DEFAULT_FADE_S` is the right
*default* — it is the crate's own "short enough to read as immediate, long enough not to click" — and it is what shipped
in iteration 3, so a consumer sees no behaviour change when the field appears.

## What is blocked

Nothing. Iteration 3 shipped the hold fix with `DEFAULT_FADE_S` as the release, which is the safe half of the same
change: the number a project wrote (`duck_s`) now does what its sentence promises, and the number it did not write yet
is the crate's own constant. If the owner prefers option 2, no code changes; if option 1, one field and one
`CHANGELOG` line in the next minor.

## A related note, for the same decision

`MusicLayers::fade_s` is accepted, stored in `Pending`, and never read: every layer move eases at `MIX_TAU_S` (50 ms).
That is **not** filed here as a question — it is a bug, the field already exists, and the fix is internal (carry the
fade into `LiveLayer`, or set the easing's time constant from it). It is iteration 4's first item.
