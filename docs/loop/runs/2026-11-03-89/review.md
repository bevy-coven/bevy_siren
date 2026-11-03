# Iteration 89 — review

## Pass 1 — Function: are #22's four criteria met?

**Critique.** Each criterion is a test, and each would fail a wrong implementation:

| # | Criterion | Test | Why it is not vacuous |
|---|---|---|---|
| 1 | a sine's centroid is its frequency within 5 % | `a_sine_centroid_is_its_own_frequency` | four frequencies 220–8000, so a constant offset fails at one end |
| 2 | white noise and its low-passed copy differ in the expected direction | `a_low_passed_copy_is_darker_than_white_noise` | asserts **centroid and rolloff** both fall |
| 3 | silence yields the format's floor, not zero, not NaN | `digital_silence_has_the_crates_floor_and_no_centroid` | exact `SILENT_DB`, and `None` rather than `0 Hz` |
| 4 | all three survive a round trip | `spectral_features_round_trip_in_both_formats` | **both** formats, and silence stays distinct from a DC offset |

**The design point worth defending** is that the centroid is an `Option`. `#22` warns that *"a fully silent clip divides
by zero in a centroid"*, and the answer chosen is not a guard-and-default but a **state**: silence has no brightness.
`0 Hz` is a *real* reading — a DC offset measures 9.375 Hz — so defaulting to zero would make the two indistinguishable
in memory *and* on disk. Both the text and binary sections write `-` for it, and the round-trip test asserts they stay
apart.

**Verdict: GREAT.**

## Pass 2 — Zoom: the two assertions that were wrong before the code

**Critique.** Both were **my** errors, and both were caught only by running the tests.

**A DC offset reads 9.375 Hz, not 0.** I asserted `Some(0.0)` and got `Some(9.375001)`. The physics is that a constant
multiplied by a Hann window is a raised cosine whose spectrum is three bins wide, so the energy-weighted mean is one bin
of 46.875 Hz divided across that spread. The measured value is now the assertion, with the leakage explained — a DC
offset is *darker than anything audible* either way, and pretending it is exactly zero would hide what the window does.

**The "short clip" fixture was five windows long.** `#22` asks how a clip shorter than the window is handled, and my
test used 0.1 s — **4 800 samples against a 1024-sample window**. The assertion passed without ever entering the branch
it was written for. The window is **21.3 ms**, and the fixture is now 10 ms, which actually exercises it.

**The general lesson**, and the third time this has happened in this loop: *a fixture that does not reach the code path
it names makes a test vacuous while looking like coverage.* Iterations 87 and 88 both had the same shape — a constant
vector, and a `seed % 24` collision.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does it cost, and can a person read it?

**Critique.** Three numbers, each with the thing `#22` demands be stated:

- `centroid_hz` and `rolloff_hz`: `Option<f32>`, and `ROLLOFF_FRACTION` is **exported** so a number never travels
  without its percentage.
- `noise_floor_db`: the method is documented as *the mean power of the quietest tenth of frames, against the clip's own
  loudest* — a percentile, which is a **design default** because no source fixes one, named so a reader can disagree
  precisely.

**The cost, measured and stated.** Decode fell from **2 636 to ~1 150 clips/s**. That is a 55 % slowdown of the scan,
and it is the case this iteration targeted, so it is not a 10 %-rule violation — but it is large enough to be the
headline of `perf.md` rather than a footnote, and the note that a build now yields three more statistics per asset is
what makes it a trade rather than a leak.

**Where the cost could go, if it ever matters**: `mel::log_mel` computes the same frames for the embedder's
preprocessing, so the two passes could share one FFT sweep. That is a measured task, not a guess, and it is named as
carved-off work rather than quietly deferred.

**Verdict: GREAT.**
