# Iteration 106 — review

## Pass 1 — function

**The defect.** `SirenSettings::problems()`'s doc said *"Twelve checks, and `tests/mix.rs::problems_names_every_mistake`
walks all twelve so this list and the code cannot drift apart"*. Measured: **thirteen** checks performed, **eleven**
needles asserted, **twelve** claimed. The check missing from every list was the `by_db == 0.0` one — a check whose own
comment records it was written for a **measured** bug.

| criterion | evidence |
|---|---|
| one list, not three | `mix::SETTINGS_CHECKS`, re-exported at the crate root; the doc names it and does not restate it |
| the list and the test agree | `assert_eq!(needles.len() + ASSERTED_ELSEWHERE.len(), SETTINGS_CHECKS.len())` |
| a check with no needle fails | **proved**: appending a fourteenth name made the test fail naming the list, then reverted |
| the drift-hidden check is exercised here | the fixture gained `Duck::new("Music","Effects").by_db(0.0).hold_s(0.25)`, and its needle passes |
| the dedicated-fixture entry is honest | `ASSERTED_ELSEWHERE` is asserted to still be in `SETTINGS_CHECKS`, so a deleted check cannot leave a stale exemption |

**The source.** Ousterhout, *A Philosophy of Software Design*, Summary of Red Flags: **Information Leakage** — *"a design
decision is reflected in multiple modules"*. The count of checks was one decision written in three places with nothing
forcing agreement, and the doc's own claim was the third copy asserting the other two matched.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/dialogue-greeting.png`.

- Full-band attack to 0.025 s, a low-frequency body, a flat floor after 0.03 s. **No vertical line**, no shelf, no hole,
  no clipping plateau. A dialogue cue is the clip a duck rides on, so it is the right one to render.
- **All 21 rendered wavs byte-identical** to iteration 105; master **−21.5 LUFS, −7.5 dBFS true peak**.
- **One image was useless and is recorded rather than shipped as evidence**: `bus-music-wave.png` is blank. `bus-*.wav`
  is a **gain over time** — a constant, by construction — so `showwavespic` draws nothing. That is not a defect in the
  audio; it is the wrong tool for a gain track, which the loop's own instructions already say (*"plot a bus's gain as a
  line … not `showwavespic`, which draws audio and turns a gain curve into an unreadable blob"*). The file is kept and
  named as such rather than quietly deleted, because a blank image that looks like a finding is worse than no image.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Read as a maintainer adding a fourteenth check:

- Add the check, add its sentence, and the test fails with `SETTINGS_CHECKS` printed in the message. The name to add is
  right there, and the failure says *"needs a needle here or a dedicated fixture"*.
- Read as a crate user: the doc no longer contains a number that can be wrong. It says *"the checks are
  `SETTINGS_CHECKS`"*, which is a pointer, not a copy.
- Read as a reviewer: the diff is one constant, one rewritten doc paragraph, and a test that now compares two counts that
  used to be independent.

**Three attempts were needed to get the count assertion right, and each failure was real information**: first 12 vs 13
because my replacement had clobbered a multi-line needle; then the `by_db(0.0)` needle failed because the **fixture had no
such duck**; then the over-budget needle failed because `max_voices: 0` in that fixture hides it behind an earlier
sentence — which is precisely what its own comment says, and why it has a dedicated fixture. The `ASSERTED_ELSEWHERE`
list exists because that fact had to be written down rather than worked around.

**One criticism, recorded:** `MusicScore::problems` claims the same shape (*"Nine checks, and this list is the whole of
it"*) and is one inspection away from the same drift. Applying this treatment there is mechanical and was carved off to
keep the finding legible.

**Verdict: GREAT.**
