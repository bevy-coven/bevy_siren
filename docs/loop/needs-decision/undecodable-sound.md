# Does a broken file get its own refusal?

## The question

`Refusal::MissingSound { sound }` now carries two causes: a key with no handle, and a key whose bytes the decoder
refused. Both print *"no handle or undecodable bytes for sound `step_03`"*, and both name the key — correct, and one
sentence for two different things. Should the decode failure be its own variant?

## The options

1. **A new variant: `Refusal::Undecodable { sound }`**, printed *"the bytes for sound `step_03` are not decodable
   audio"*. Cost: an **exhaustive `match` on `Refusal` in a consumer breaks** — the same cost as any new variant in a
   `0.x` crate, and a host that renders refusals must add an arm or a `_` fallback. Benefit: a game can react
   differently — "you never mapped this key" is an authoring bug to fix now; "this file is corrupt" may be a bad
   download to retry, and a panel can style them differently. `MissingSound`'s sentence goes back to one cause and
   becomes simple again.
2. **Keep one variant, split the sentence by cause.** Cost: none — but the sentence cannot know the cause, because the
   variant carries only the key. It is what shipped, and it is why both lines read the same.
3. **A `cause` field on `MissingSound`: `{ sound: String, cause: MissingSoundCause }`.** Cost: a breaking change for
   anyone destructuring with named fields; and a field, unlike a variant, cannot be `#[non_exhaustive]`-checked at
   compile time, so a consumer's `match` on it would be a wildcard and not an error when a third cause appears.
   Benefit: one variant, two facts.
4. **Log the cause, refuse with one variant.** Cost: none to the API; the `Entry::Refused` line gains it. Benefit: a
   person reading the panel's log learns which it was, while a program sees one shape. Limitation: a game's own meter
   cannot branch on it.

## The recommendation

**Option 1, in the next minor version**, with option 4 as the cheap interim. The crate's premise is that a refusal
names what is missing, and *what to do about it* differs between these two: a missing key is a line to add to
`SirenSounds`, a corrupt file is a file to re-export. Option 3's field is the worst of both — a break without the
compiler's help when a third cause appears. Option 2 (shipped) is true and actionable; it simply cannot be acted on
differently.

If the owner prefers no break, option 4 is one line in `playback.rs` and makes the panel's log line distinguish the
two without touching the message.

## What is blocked

Nothing. Iteration 6 shipped the key and a sentence true for both causes, which needed no public change. The
distinguishing variant waits on this answer.

## Iteration 116: a third cause, and option 4 shipped for it

`MissingSound` now has **three** causes: a key with no handle, bytes the decoder refused, and **a file that failed to load** (`LoadState::Failed`: a typo in a name, a file nobody delivered). Until iteration 116 the third was not
a refusal at all — every play of it was answered `Played` and nothing sounded (`runs/2026-11-04-116/`). It is refused now, at the play and for a voice already in flight, with option 4's shape: **one variant in the message, the
cause and the path in the log** (*"the file for sound `x` failed to load: Path not found: /…/x.ogg"*). The limitation stands and is the argument for option 1: a game's own meter that prints `Refused::reason` reads the one sentence
for all three, and **the three want different things from a project** — a missing key is a line to add, a corrupt file is a file to re-export, a failed load is a name or a path to fix, and only the last two have the server's own
words to say. The recommendation is unchanged and a little stronger: **option 1, a variant, in the next minor version** — now for two new variants (`Undecodable`, `LoadFailed`), with `MissingSound` going back to one cause.
