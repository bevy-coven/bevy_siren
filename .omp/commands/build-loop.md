---
description: One iteration of the bevy_siren improvement loop
argument-hint: "[optional focus, e.g. 'occlusion' or 'dialogue queue']"
---

One run of this command is one iteration of a long-running loop on bevy_siren, a Bevy 0.20 crate that gives a game
sound worth having: a cue table as data, buses that duck, a voice budget that refuses politely, layered music that
changes on the bar, and a log of what played and why. Do ONE improvement per iteration, the single most valuable one
from a game developer's point of view, finish it completely, then stop.

README and docs follow https://github.com/bevy-coven/.github/blob/main/DOCUMENTATION.md; a feature is not done until
its README line and, if visible, its GIF exist.

This crate must be **easy, right, and cheap**: a cue a game cannot play in one line is not done, a mix that disagrees
with its own log is not done, and a system that scans every voice every frame for nothing is not done. Its API is the
family's: flat re-exports and no prelude, independent plugins, systems in `SirenSystems`, settings as serde data,
engine-free maths in `model`. Its design rules are in `docs/research/siren.md`.

You do all the work yourself, including looking at the images and judging them. Do not delegate visual review.

Extra instructions for this run (a focus such as "occlusion"): $ARGUMENTS
They override the choice of item in section 4, not the gates.

State lives in `docs/loop-log.md`; rules and traps live in `AGENTS.md`. Follow them; do not re-derive them.

## Every pass ships: commit, push, publish, docs, demos

1. **Commit and push.** Everything the pass made is committed and pushed to `main` (through the gate) before the pass
   ends. Nothing stays only in the clone.
2. **Publish.** A pass that changed what the package ships releases it in the same pass. A publish crates.io refuses
   with `429` is logged once and not retried in this pass.
3. **Docs.** A change to behaviour or API updates its documentation in the same pass: the README line, the
   `docs/guide/` section, rustdoc and the CHANGELOG. Images and links in a README are absolute URLs.
4. **Demos.** A change anyone can see ships its demo in the same pass: a GIF or still recorded from `examples/mixer`'s
   capture knobs, shown in the README or `docs/guide/`, and listed in `docs/media/README.md`.
5. **Read the last audit first.** Anything the previous pass left undone (`push`, `release`, `docs`, `demo?`) is this
   pass's first job, ahead of its item; only a broken gate comes before it.

## 1. Orient

1. `AGENTS.md`: rules, traps, gates.
2. `docs/loop-log.md`: what earlier iterations did, found and deferred, and the newest iteration's ranked backlog. The
   next iteration number is the last one plus one.
3. `git log` since the last logged iteration.
4. The `docs/research/<system>.md` note of any system you may touch.
5. `git status --short`. If you find tracked changes you did not make, stop and log them; do not commit or revert them.

Before choosing an item, triage the repository's issues:

```sh
gh issue list --repo bevy-coven/bevy_siren --state open --limit 100 --json number,title,labels,createdAt \
  --jq '.[] | "#\(.number) \(.title)"'
```

Then in this order: a `bug` issue; a `consumer` issue (a game or the Cauldron is waiting); any other open enhancement,
oldest first. Only when all are done, blocked or released does section 4 choose a feature of your own. A blocked issue
gets a `gh issue comment` saying exactly what it needs and is skipped; it never stops the pass. An empty issue list
costs nothing: write one line saying so and continue.

## 2. Capture the current state

```sh
cargo build --release --examples
mkdir -p /tmp/sN/before
SIREN_FRAMES=200 SIREN_SHOT=/tmp/sN/before/mixer.png SIREN_KEYS=duck,steal,layer SIREN_KEY_EVERY=20 \
  SIREN_SIZE=1280x720 target/release/examples/mixer
```

Record the `siren: frames … mean_ms …` line: it is the performance baseline. Anything a state change shows (a duck, a
steal, a layer entering) needs the state staged before the shot: add a `SIREN_KEYS` token rather than describing it
unseen.

## 3. Inspect closely (you judge the images yourself)

Read each image with `read` and zoom in. Quote pixel values when a number settles it. Always view one actual-resolution
full frame. Write the **audio-panel checklist** to `/tmp/sN/findings.md`, each line with a measurement or a file and
region:

- each bus row's dB against `SirenMix` for that frame, and the duck against the `Duck` rule;
- each bus row's voice count against the live `Voice` count;
- the log pane against the `SirenLog` tail (a row that does not match a `Played`, `Refused` or `Stopped` is a defect);
- a stolen voice visible as a `Stopped { why: "stolen by …" }` line and a bus at its cap;
- the music layer's entry on a bar, from the printed transition time against the tempo.

## Owner input: recommend, record, proceed

**This loop may run automated.** When a pass needs the owner's input (a design choice, an API shape, a default), it
answers the question itself:

1. **Recommend.** Name the two or three real options and pick one, with one line of why. Prefer the conservative
   choice: reversible, conventional, consistent with the existing code and the issue's stated intent.
2. **Assume the owner chose it, and build it this pass.** A decision is never a reason to skip an item or end a pass.
3. **Record it where the owner reviews.** On the issue, a comment beginning `Assumed decision (owner may override):`
   with the options, the pick and why, and the label `assumed-decision`; with no issue, in the commit body and the log
   line, beginning `Assumed decision:`.
4. **The owner's reply wins.** An owner comment that contradicts an assumed decision overrides it.
5. **Protected checks change only additively.** An assumed decision may add to a test or a check; it never removes,
   loosens, skips or re-pins an existing assertion.
6. **Never, whatever you assume.** Yanking a release or changing crate ownership; force-pushing, rewriting history or
   deleting branches; deleting files or work this pass did not create; editing this command file; spending money or
   sharing credentials.

## 4. Pick the single most valuable item

Rank candidates against the question "what stops a game from shipping its sound?":

1. A cue that cannot play when it should, or plays the wrong thing.
2. A mix that disagrees with its log, or a budget that grows past its promise.
3. A setup that takes more lines than it should.
4. A sound feature a shipped game expects and this crate lacks: occlusion and a low-pass, a dialogue queue with
   subtitles, reverb zones, a probe for an editor's audition.
5. Frame-time cost: a per-frame scan that could be a change flag.

State in one sentence why your pick beats the runner-up, and record the ranked list in the log.

## 5. Research thoroughly (before any code)

Every feature gets researched, with no exceptions, and every iteration cites three kinds of source: a **craft** source
(a shipped game's audio talk or a paper on game audio), a **shipping-game** source, and a **technical** source (DSP,
mixing, Bevy's or rodio's own source). Write or extend `docs/research/<system>.md` with `## Sources`,
`## Craft principle`, `## How shipping games do it`, `## Model adopted`, `## Parameters`, `## Validation targets`,
`## Deviations`, `## Pending index`, and quote the sources rather than paraphrasing from memory. Commit the note alone
before writing code.

## 6. Implement within the crate's rules

- `src/model.rs` names no engine type; a test enforces it.
- Every play answers exactly one `Played`/`Refused`; a new refusal is a variant with what is missing.
- One writer per voice gain: `update_mix`. The music moves targets, never sinks.
- The two backends answer identically: a new test must pass with and without `--features mixer`.
- `--features serde`: a new field on `CueTable`, `SirenSettings` or `MusicScore` needs `#[serde(default)]`, a sentence
  in `problems()` if it can be wrong, and a line in the guide.
- No `unwrap()`/`expect()` on caller data; no game vocabulary.

## 7. Prove it

- A failing-first test in the one `all` binary, then the fix, then the test green.
- Run the example: `SIREN_FRAMES=200 … cargo run --release --example mixer`, read the image, judge it.
- The gate, in this order: `cargo build`; `cargo test`; `cargo test --features serde`; `cargo test --features mixer`;
  `cargo test --doc`; `cargo build --examples`; `cargo clippy --all-targets --all-features -- -D warnings`;
  `cargo fmt --check`; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.

## 8. Ship, and log

Commit and push; release if the package changed; update the docs and the demo; write the iteration into
`docs/loop-log.md` with what shipped, what was measured, the captures, and a ranked backlog.

## One test binary, not N

Every file in `tests/` is a module of `tests/all.rs` (`autotests = false`), and `every_test_file_is_a_module` fails on
an undeclared file. A new test file is a `mod` line there, or it never compiles.

## The two-way street, both ways

**A consumer that needs something files it here, and this loop builds it here.** A game or the Cauldron opens an issue
on `bevy-coven/bevy_siren` labelled `enhancement` (or `bug`), `consumer` and `consumer:<project>`, titled `S-NN: <what>`
with the next free `NN` after the highest `S-` title, whose body says why, the proposed API, the edge cases, and an
acceptance a test in this repository can prove. An open `consumer` issue is taken right after bug issues and ahead of
the owner's focus; the fix is made here, in its own commit with its own test, as the general form (a project is the
example, never a name in the API), and the issue is closed only by that commit's `Fixes #N`.

**This crate's own needs go the other way.** When a cue, a bus rule, a music feature or a field of `SirenSettings`
cannot do what the Cauldron's audio panel needs, the request is filed on `bevy-coven/bevy_grimoire` labelled
`enhancement`, `consumer` and `consumer:bevy_siren`, and the editor item waits with `Needs: bevy_grimoire#N`. Never
patch, wrap or vendor the editor here.

Both directions carry "recommend, record, proceed" and the `assumed-decision` label, so an unanswered design question
never idles a loop.
