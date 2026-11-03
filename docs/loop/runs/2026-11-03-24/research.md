# research.md — iteration 24: a refusal lives 0.4 s in a busy scene

## The item

`SirenLog` is the crate's record of *what played and why*, and its reason for existing is the silent failure: a cue
that never plays and never says why is the hardest thing in a game to find. The buffer is 256 entries and `push` evicts
the **oldest** when full. Every entry is equally evictable, so one refusal is pushed out by 256 routine plays:

```
push(Refused { cue: "typo", reason: "no cue of that name" })
then 256 × push(Played { cue: "c_i" })
  -> the refusal is gone (measured exactly: gone after 256 plays)
```

A 32-voice scene at 20 footsteps a second writes 640 entries a second, so a refusal survives **~0.4 s** — and that is the
moment a developer is looking. The line that explains why something did not play is evicted by lines saying something
did.

## Sources

**S1. R6 (vendored: `docs/research/siren.md`)** — *"a play is never silent: `Played` or `Refused`, both with the reason"*,
and the crate's own **fail loud** rule: *"a refusal names what is missing; nothing plays on a refusal."* A refusal that is
**written and then thrown away within a second** is a refusal a person cannot read, which is the silent failure with an
extra step.

**S2. `SirenLog::CAPACITY`'s doc** — *"How many entries the buffer keeps. A panel shows eight; a test reads one; **the rest
is for a person asking what just happened**."* The stated use is a person asking *after* an event. A ring that keeps the
last 256 of a uniform stream answers "what just happened" only for the last 0.4 s.

**S3. Gregory, *Game Engine Architecture* 3e, §14.5.8.4** (vendored: `siren.md` R4) — the same shape as a **virtual
voice**: a refusal is a *logical* fact the engine must keep even when the physical channel is busy. The crate already
uses that language for voices; the log is the same resource class.

**S4. `Entry`'s own variants** — `Played`, `Refused`, `Stopped`, `Ducked`. Three of them are **routine** at frame rate
(a busy scene plays, stops and ducks constantly); `Refused` is **exceptional** — a healthy game produces none. A buffer
that treats them alike is optimising for the wrong stream.

## The technique

**Evict a routine entry before an exceptional one.** When the buffer is full, the entry to drop is:

1. the **oldest routine** entry (`Played` / `Stopped` / `Ducked`), or
2. if there is no routine entry at all — every one of the 256 is a refusal, which means 256 things went wrong and a game
   is already broken — the **oldest** entry, whatever it is.

`Refused` is the exceptional class. Keeping it costs at most the routine entry that would have been dropped anyway, and
it means a panel shows the refusal **alongside** the plays it interrupted, in order, for as long as it takes 256 *routine*
entries to accumulate rather than 256 *total* ones.

**Bounded, and no new field.** The buffer stays 256 entries: the same memory, the same `recent(n)`, the same `iter()`. Only
**which** entry leaves changes.

**Why not a second buffer or a `Refused`-only ring?** That is a new public type, a new accessor, and a panel that has to
read two streams and merge them by `at_s` — a second set of rules for one list. The crate's own trap list is about
exactly that.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a refusal names what is missing; nothing plays on it | — | S1 |
| the log's 256 entries are *"for a person asking what just happened"* | 256 | S2 |
| a logical fact outlives its physical channel | — | S3 |
| `Refused` is exceptional; the other three are routine | — | S4 |

## What good sounds like

A game with a burst of footsteps **and** a mistyped cue shows the mistyped cue's line in the panel after the burst, in its
original position among the plays, until 256 *routine* entries have happened since — which in a busy scene is minutes
rather than the 0.4 s it is today. A log with nothing wrong looks exactly as it does now: refusals are rare, so the
eviction rule almost never fires.
