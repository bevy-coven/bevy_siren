# review.md — iteration 7, three passes

## Pass 1 — function

**The claim.** `MixerBackend` reuses a voice slot whose voice has ended, so the per-bus slot list is bounded by the
**peak concurrent** voice count, not by the number of sounds a game has played; and a reused slot is a **new name**,
so a `MixerVoice` held by an entity the world has not finished despawning addresses nothing.

**Against the sources.** Gregory §14.5.8.4 calls the voice count **fixed**; the backend kept one slot per sound ever
played, so "fixed" grew without bound. The crate's rule R4 bounds *concurrent* voices — that promise held and still
holds; what was unbounded is the allocator's copy of it. `rodio`'s `Player` holds three `Arc`s and a queue
(`player.rs:20–28`), and `Player::stop` frees nothing — dropping the `VoiceTrack` is what releases them, which the
fix does by overwriting the slot.

| measurement (real device, `MixerBackend::open`) | before | after |
|---|---|---|
| 400 one-shots, all ended: slots held | **400** | **6** |
| 200 one-shots: slots held | 200 | ≤ 32 (asserted; 2 measured) |
| 200 one-shots: distinct indices handed out | 200 | 200 — each hand-out is a fresh *name* |
| a stale index's `finished` / `set_voice_db` / `stop_voice` | addressed the slot's new occupant | a no-op; `finished` says ended |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The second row is the one worth reading twice.** Distinct *indices* stay at 200 in both trees: the generation bump is
deliberate, and it is what makes a reused slot safe. So the evidence is not "fewer indices" — it is **fewer slots and
the same number of names**, which is the only combination that both bounds memory and cannot cross a wire.

**Verdict: GREAT.** The leak iteration 2 found and deferred is closed, the hazard its fix creates is closed in the same
place, and both are measured on the real backend rather than a fake.

## Pass 2 — zoom

**Read:** `evidence/{before,after}/` cue WAVs, spectrograms, `render.txt`, and the two probe runs (400 plays, real
device).

**One defect in my own work, found and removed.** The first probe I wrote for `examples/budget` used a fake output that
hands out its own indices — so it measured the *crate's entity side*, which was never the leak, and would have printed
"distinct indices rises with plays" on both trees. It proved nothing about `MixerBackend` and was deleted. The
measurement that matters is the real backend's, which this machine can open: `probe7b` (deleted after use) printed
`400 plays: distinct indices = 400, slots held = 6` after the fix and `… slots held = 400` before it, with the same
binary shape.

**No audio defect can exist here**, and the check confirms it: 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS /
−12.1 dBFS looped past the gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS, the spectrum decaying to −120 dBFS with no
vertical edge at the 35 ms cutoff.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The slot-reuse test drives 200 one-shots on a real device and asserts three things at
once — slots bounded, samples actually started, and every hand-out a fresh name — so a fix that bounded slots by
*reusing indices* would go red on the third. The stale-index half calls all three index-taking methods with a dead
name and requires them to be silent.

**Can a developer understand it without the code?** The numbers are `100 plays → 200`, `400 plays → 6 slots`, and
`distinct indices = 400` in both trees. `MixerBackend::voices()`'s doc now says *slots*, not *played*, which is the one
place a reader could have been misled.

**What a fresh reader would still ask:** whether the O(slots) scan in `play` costs more than it saves. It scans a vector
bounded by `max_voices` (32 by default) and stops at the first free slot, in a method that already allocates a
`rodio::Player`; the scan is not the cost that matters, and `perf.md` says `perfcheck` cannot see it either way.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a probe that measured the wrong subsystem);
deleted.
