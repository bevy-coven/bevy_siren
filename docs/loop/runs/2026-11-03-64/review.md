# review.md — iteration 64, three passes

## Pass 1 — Function: does decoding do what the file says?

**The signal survives.** `the_decoded_audio_is_the_tone_that_was_written` writes a 440 Hz tone and counts **exactly
220** rising zero crossings in 0.5 s. A decode that returned noise, silence or a shifted rate would fail; a length
check would not.

**The rate is the stream's.** `a_wav_decodes_to_its_own_rate_and_channels`: 48000 / 1 / 4800 samples / 0.1 s.

**The commonest rate gets the truthful message.** `a_file_at_another_rate_is_decoded_and_then_refused_naming_both_rates`
proves decoding **succeeds** at 44.1 kHz and the refusal is `ResamplingRequired { from: 44100, to: 48000 }` — about
conversion, not about reading. Both rates are in the string.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Two defects, one mine and one the compiler's:

| Defect | Found by | Fix |
|---|---|---|
| my zero-crossing bound was `(436..=444)` — **vague where the number is exact** | the test itself, which got 220 | an equality: 440 Hz × 0.5 s is exactly 220 crossings |
| `u16::try_from(decoder.channels().get())` — a **useless conversion**, because `channels()` already returns `u16` | clippy | removed; the call is one line |

The first is the more interesting: I had written a tolerance around a number that has no tolerance, which is how a
test stops proving anything.

**A third, caught by the pin rather than by me:** `the_semantic_feature_declares_no_dependency_of_its_own` failed when
`semantic` grew `dep:rodio`. That is the test doing its job — it is the crate's ratchet on a published promise.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use this without me?

- **The messages were read.** *"`/nonexistent/nope.wav` is not audio this crate decodes: …"* — names the file and
  passes the decoder's own words through. *"`cd.wav` is not audio this crate decodes"* is **not** what a 44.1 kHz file
  gets; it gets *"the file is 44100 Hz and the model needs 48000 Hz"*, which is the distinction this iteration exists
  to make.
- **The default build is untouched**: 413 dependency lines either way, 174 tests green with no features.
- **A 44.1 kHz library's experience** is: every file decodes, every file is indexed for its metadata, and the
  *embedding* step refuses with a message that names the decision file. Loud, once, with the reason.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| issues | **#12 advanced** — the decode half built; the conversion carved off to a measured decision |
| new module | `src/semantic/decode.rs` |
| tests | **+6**, from 265 to **271** all-features |
| perf | **first measured baseline** — 0.024 / 0.284 / 0.667 µs per frame at 8 / 64 / 256 |
