# eval.md — baseline and the `cues.yaml` table, iteration 29

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 112 + 5 doctests green |
| `cargo test --features serde` | 119 + 5 green |
| `cargo test --features mixer` | 121 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 %). No open issues. Iteration 28's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. No case reads `docs/`.

## The lead, and what it became

Iteration 28's lead was a **stale `set_voice_db`** after a mixer slot is reused. Probed, **clean**: `set_voice_db` goes
through `occupant_mut`, which compares the packed generation, and the crate's own
`a_slot_is_reused_and_a_stale_index_addresses_nothing` already asserts it (line 382: `backend.set_voice_db("Effects",
stale, -80.0)` then the live occupant is unchanged). So is `finished` versus `play`'s free-slot predicate — both read
`stopped || player.empty()`, so they agree.

Three more probes came back clean: every `Attenuation` model at its documented distances (`Inverse` −6.02 dB at twice
`near_m`, `Linear` exactly 0 at `max_m`, `None` flat, and every guard falling back as documented); a settings swap
**mid-duck** (both `db` and `ducked` prune to the new bus list, `deepest_duck` names only a real bus); and the mixer's
slot bookkeeping after iteration 28.

The probe that paid was on the **front door**:

| artefact | before |
|---|---|
| `docs/media/mixer.png` | generated 2026-11-02 14:40, **before** the `Problems` block; alt text says "eight cues … eight log lines" |
| `docs/media/mixer.gif` | same date, same staleness |
| `docs/media/mixer-panel.gif` | created by iteration 26, named in **no** README |

## Ranked

1. **The README shows a panel the code no longer draws, described in wrong counts** — the crate's front door, and nothing
   in `cargo test` can see it. **The pick.**
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, six items deep).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| artefact | after | green? |
|---|---|---|
| `docs/media/mixer.png` | regenerated from the clean table: nine cues, `Problems` / `none` | green |
| `docs/media/mixer.gif` | regenerated with `SIREN_FAULTS=1`: two rows `<- problem`, the block naming both | green |
| `docs/media/mixer-panel.gif` | deleted (orphan) | green |
| `check_media.py` | 6 ok, exit 0; **reports the orphan and exits 1** on a pre-fix reproduction | green |
| every test, both feature sets | unchanged (112 / 119 / 121 + 5) | green |
