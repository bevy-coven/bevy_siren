# 0006 — One crate, two front ends: a plugin and a tool server over the same library

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#7](https://github.com/bevy-coven/bevy_siren/issues/7)
- **Supersedes:** nothing. **Depends on:** [0002](0002-inference-runtime.md)

## The question

`bevy_siren` is a Bevy plugin: it owns resources in an `App` and its systems drive playback. The semantic feature must
also be a set of agent tools, and this project's convention is that a crate's tools live in that crate's repository. So
the crate becomes two front ends over one library — and they must agree: a tool that says "play this track" has to
reach the same cue table, the same buses and the same index the plugin's systems read.

The failure mode to design against is **two divergent worlds**: an index the tools build and the plugin never sees, or
a play the tools request that no `App` receives.

## Decision

**One library crate, one `[[bin]]` behind the `semantic` feature, and no second copy of any state.**

```
bevy_siren/                   the library — plugin, cue table, buses, music, retrieval
  src/semantic/               index, retrieval, ranking. No engine type in the arithmetic.
  src/bin/siren-tools.rs      the MCP server, behind `semantic`
tests/                        one binary, as today
```

A workspace was considered and rejected: the crate is at 0.1.0, and splitting it now would publish a second crate name
that has to be kept in step. A `[[bin]]` behind a non-default feature is invisible to a user who only wants playback.

## The seam: one direction of data, two of control

```
        ┌──────────────┐         ┌──────────────────┐
        │  the library │◄────────│ the tool server  │  reads the index, builds it, queries it
        │  (index,     │         │ (MCP)            │  NEVER owns playback
        │   ranking)   │         └──────────────────┘
        │              │
        │              │◄────────── Play / MusicLayers messages
        │  the plugin  │         ┌──────────────────┐
        │  (an App)    │◄────────│ the Cauldron     │  in-process, no protocol
        └──────────────┘         └──────────────────┘
```

**The index is owned by the library, not by either front end.** Both read it; the tool server writes it; the plugin
reads it when it needs to answer "what would this play". There is no second index and no cached copy in the `App`.

**Playback has exactly one path, and it is the plugin's.** The tool server never writes a gain, never creates a sink and
never starts a voice. It sends the messages the plugin already drains — `Play`, `MusicLayers`, `Stinger` — and the
plugin answers `Played` or `Refused` as it does for any other caller. That keeps the crate's rule intact: **one writer
per voice gain**, and **every play answers exactly one of `Played`/`Refused`.**

## How a tool call reaches a running `App`

Numbered, because this is the seam that fails silently if it is guessed:

1. A tool call arrives at `siren-tools`.
2. The server validates the arguments and queries the library's index — no `App` needed.
3. To audition, the server sends a `Play` message **through a handle the plugin installed in the library**, or, where
   the plugin is in another process, writes the request to the file the plugin watches.
4. The plugin's existing `play_requests` system drains it in `SirenSystems::Play`.
5. The answer — `Played` or `Refused` — is appended to `SirenLog`, which the server reads back.

**Step 3 has two implementations and only one is built now.** In-process is built first because the Cauldron is
in-process; the cross-process file hop is a later issue, filed when a server needs to drive a running editor.

## The headless case, answered

With no running plugin the server can still:

- build an index, query it, and report scores and breakdowns;
- report what a cue *would* play, by reading the cue table and the library — the decision, not the sound;
- say why a play would be refused, from the same `problems()` and refusal logic the plugin uses.

It cannot: play audio, report a voice count, or observe a duck. **A tool that would need those must say so rather than
pretend**, which is the crate's fail-loud rule applied to a process boundary.

## Cost to a consumer

| | |
|---|---|
| default build | unchanged — the binary and its dependencies are behind `semantic` |
| a user who enables it | gets a `siren-tools` binary alongside the library |
| a panel author | calls the library directly; no serialisation, no protocol |
| `tests/leaf.rs` | the binary's dependencies must be argued for in `S-09` like any other |

## What would reverse this

A requirement that the tool server run on a machine with no Bevy at all — then the library must be split so the server
does not link an engine. Nothing in the target workflow needs that today.
