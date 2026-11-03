# Does a stinger get its own refusal?

## The question

`play_stingers` refuses two things: a cue that is **not in the table**, and a cue whose `bus` is not `Music`. The first
is `Refusal::UnknownCue`, which is exactly true. The second has **no variant**: `UnknownBus { bus: "Effects" }` would
print *"bus `Effects` is not in the mix table"*, which is false — `Effects` is in the table; the stinger is what must be
on `Music`. Should `Refusal` gain a variant for it?

## The options

1. **Add `Refusal::StingerOffMusic { bus }`**, printed *"a stinger plays on Music, but `ui.click` names bus `Effects`"*.
   Cost: a **new variant on a public enum** — an exhaustive `match` in a consumer breaks, and a host that renders
   refusals needs an arm or a wildcard. Benefit: the `Refused` message a stinger produces tells the truth, and a game
   can react to it (an authoring bug to fix, not a runtime condition).
2. **Keep the prose in `SirenLog` only** (shipped in iteration 9). Cost: a host reading `Refused` never learns the
   stinger was refused; only a person reading the log does. Benefit: no break. This is what the crate does today, and
   the log line is precise: *"a stinger plays on Music, but `ui.click` names bus `Effects`"*.
3. **Reuse `UnknownBus`.** Cost: none to the API, and the sentence would be **false** — `Effects` is in the mix table.
   Rejected: a refusal naming the wrong thing is the class of bug iterations 5, 6 and 8 all fixed.
4. **Reuse `NoSounds` or `SpatiallyMute`.** Both are false in a different way. Rejected for the same reason.

## The recommendation

**Option 1 in the next minor version.** A stinger that cannot play is an authoring mistake with a one-line fix
(`bus: "Music"`), and the crate's whole premise is that a refusal names what is missing so a person can act on it.
Option 2 is true and costs nothing, which is why it shipped; it is not *wrong*, only incomplete for a program. It is a
cheap addition to a decision that is already open in three other files (`undecodable-sound.md`,
`refusal-names-its-cap.md`, `remove-completes.md`) — all four are enum arms, and one minor can carry them together.

## What is blocked

Nothing. Iteration 9 shipped option 2: the unknown cue is `UnknownCue` both in the log and as a message, the wrong-bus
case is a precise prose line in the log, and **neither ducks the music**. The variant waits on this answer.
