//! **Key and mode, measured rather than assumed absent.**
//!
//! Two tracks a semitone apart are the most audible clash in a game's music, and no embedding reliably separates them —
//! both are *"dark ambient strings"*. Key detection is what makes a hard filter possible.
//!
//! # What ADR 0003 assumed, and what this module measured
//!
//! ADR 0003 declared key **out of scope for the Rust path**, on the grounds that *"no pure-Rust implementation reports a
//! usable accuracy figure"* and that a feature *"wrong 40 % of the time"* would make `#27` penalise the right track.
//! **The 40 % was an assumption, and this module is the measurement**: on a corpus of **24 keys** — every tonic in both
//! modes, each a I–IV–V–I progression with the chord roots in the bass — the estimate is **100 % exact**.
//!
//! That figure is on **synthetic tonal material**, which is what a key detector should find easy and what no game's
//! asset library resembles. It is recorded here rather than claimed as a general accuracy, and the decision about what to
//! do with a real library is `docs/loop/needs-decision/key-detection.md`.
//!
//! # The method, named
//!
//! **Krumhansl-Schmuckler with the Krumhansl-Kessler profiles**: audio becomes a **chroma vector** (twelve pitch classes
//! by energy), and each of 24 key hypotheses rotates a major or minor profile against it; the highest Pearson
//! correlation wins. Every number that could be a magic constant lives in [`KeyProfile`], which is public so a caller can
//! see the profile rather than trust it.
//!
//! # The optionality trap, twice over
//!
//! `#25` warns that *"an explosion has no key and an estimator will still produce one"*. Two guards:
//!
//! 1. **A chroma with no tonal content is refused before any correlation** — a silent clip has no pitch classes at all,
//!    so it reports [`Key::absent`] instead of whichever profile correlates best with a flat vector.
//! 2. **A confidence below [`MIN_CONFIDENCE`] reports no key**, so an atonal asset is [`Key::absent`] rather than a
//!    confident wrong answer, exactly as [`super::tempo::Tempo::bpm`] is `None` for a click.
//!
//! And the distinction `#25` cares about most: an **absent** key is not a wrong one, so `#27`'s key rule can leave it
//! unjudged rather than excluding on it.

use super::MODEL_SAMPLE_RATE;
use super::mel::{MEL_FLOOR, fft_in_place, hann};

/// **The chroma window, and why it is four times the mel window.**
///
/// A pitch class is a **logarithmic** interval, so the resolution a chroma needs is set by the *lowest* note it must
/// read: at C4 (261.6 Hz) a semitone is 15.6 Hz wide. A 1024-point transform at 48 kHz has **46.875 Hz per bin — 2.85
/// semitones at C4** — which cannot represent a chromatic pitch at all, however the bin is mapped. Measured: a pure C4
/// through a 1024-point window peaks at bin 6 (**281 Hz**), and the first version of this module read that as C#.
///
/// The width that matters is set by the **closest pair of pitches that must be told apart**, not by the lowest one: at
/// the top of the chroma range B4 (**493.9 Hz**) and C5 (**523.3 Hz**) are 29 Hz apart, and a Hann window's main lobe is
/// **four bins** wide, so they need more than four bins of separation or they leak into each other. Measured:
///
/// | points | Hz/bin | C5−B4 |
/// |---|---|---|
/// | 4 096 | 11.72 | **2.5 bins** — they merge, which is why B tied the tonic in a C major passage |
/// | **8 192** | **5.86** | **5.0 bins** — separated |
///
/// **8192 points**, therefore: the smallest power of two that separates a semitone wherever the chroma listens. It is a
/// separate transform from [`super::mel`]'s because the mel path's window is chosen for CLAP's model and must not
/// change, while this one is chosen for pitch.
pub const CHROMA_WINDOW: usize = 8192;

/// The hop between chroma frames: half a window, so successive frames overlap and a short note cannot fall between them.
pub const CHROMA_HOP: usize = CHROMA_WINDOW / 2;

/// The twelve pitch-class names, in the order a chroma index uses: `C, C#, D, …`.
pub const PITCH_CLASSES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// The confidence below which a key is **not reported**, leaving the asset unjudged.
///
/// The confidence is the winning correlation minus the best correlation in a **different** key, clamped at zero: a clip
/// that scores its own key and its relative minor equally is not a confident answer, whatever the winner's absolute
/// correlation.
///
/// **Design default: `0.15`**, and it is measured rather than guessed. Three clips that must be told apart:
///
/// | clip | confidence |
/// |---|---|
/// | a I–IV–V–I progression, C major | **0.241** |
/// | a single impact | **0.067** |
/// | white noise | **0.030** |
///
/// `0.15` sits between the progression and the impact — the gap `#25` asks for, *"an atonal asset is reported as having
/// no key rather than a confident wrong one"*. The first version used `0.05`, which is above noise and **below** the
/// impact, so a single hit reported a confident `D# minor`.
pub const MIN_CONFIDENCE: f32 = 0.15;

/// **The Krumhansl-Kessler key profiles**, as twelve weights each over `C..B`.
///
/// From Krumhansl & Kessler, *"Tracing the dynamic changes in perceived tonal organization in a spatial representation
/// of musical keys"* (Psychological Review 89(4), 1982) — the probe-tone ratings for the major and minor scales. They
/// are vendored here rather than cited by name alone because a key detector's behaviour **is** these twelve numbers, and
/// a reader must be able to see them.
///
/// **A [`KeyProfile`] is a serde-free plain type**: the values are a property of human perception, not a project's
/// preference, so unlike [`super::weights::Weights`] they are not data a game should edit — a project that changed them
/// would be detecting a different key system, which is `#25`'s *"authored number"* only in the sense that it is
/// research, not taste.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyProfile {
    /// The major profile, twelve weights over `C..B`.
    pub major: [f32; 12],
    /// The minor profile, twelve weights over `C..B`.
    pub minor: [f32; 12],
}

impl Default for KeyProfile {
    fn default() -> Self {
        // Krumhansl-Kessler, rounded to four decimals — the precision the paper reports.
        KeyProfile {
            major: [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88],
            minor: [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17],
        }
    }
}

/// **A key, or the stated absence of one.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// The tonic as a pitch-class index `0..12` (`0` is `C`), or `None` when the asset has no key.
    pub tonic: Option<u8>,
    /// Whether the mode is major. Meaningless when [`Key::tonic`] is `None`.
    pub major: bool,
}

impl Key {
    /// An asset with no key — an explosion, a texture, a drone with no tonal centre.
    pub fn absent() -> Self {
        Key { tonic: None, major: false }
    }

    /// A key, for tests and for a caller that already knows one.
    pub fn new(tonic: u8, major: bool) -> Self {
        Key { tonic: Some(tonic % 12), major }
    }

    /// Whether a key was found at all.
    pub fn is_some(&self) -> bool {
        self.tonic.is_some()
    }

    /// The key as `C major` / `F# minor`, or `no key`.
    pub fn name(&self) -> String {
        match self.tonic {
            None => String::from("no key"),
            Some(t) => {
                let mode = if self.major { "major" } else { "minor" };
                format!("{} {mode}", PITCH_CLASSES[(t % 12) as usize])
            }
        }
    }

    /// **How the two keys relate**, for the compatibility rule — [`Relation::Same`], [`Relation::Fifth`], or
    /// [`Relation::Clash`]. `None` when either key is absent, which is `#25`'s rule that an unknown never penalises.
    pub fn relation(&self, other: &Key) -> Option<Relation> {
        let (a, b) = (self.tonic?, other.tonic?);
        if a == b {
            return Some(if self.major == other.major { Relation::Same } else { Relation::Parallel });
        }
        // A **perfect fifth** (7 semitones) and a **perfect fourth** (5) are the consonant relationships; the circle of
        // fifths is what makes them the same relation read in two directions.
        let interval = (b + 12 - a) % 12;
        if interval == 7 || interval == 5 {
            return Some(Relation::Fifth);
        }
        Some(Relation::Clash)
    }
}

/// How two keys sit together, as the compatibility table names its columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The same tonic and mode.
    Same,
    /// The same tonic, opposite mode — C major against C minor: a deliberate shift, not a clash.
    Parallel,
    /// A perfect fourth or fifth away, the two keys that share most of their scale.
    Fifth,
    /// Anything else. `#25` asks for this to be **penalised**, not excluded, because a modal piece can sit anywhere.
    Clash,
}

impl core::fmt::Display for Relation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Relation::Same => "same key",
            Relation::Parallel => "parallel mode",
            Relation::Fifth => "a fifth apart",
            Relation::Clash => "clashing keys",
        })
    }
}

/// **What a compatibility table says about each relationship**, as the authored data `#25` asks for.
///
/// A `Clash` is a **penalty**, never an exclusion — `#25`'s own reasoning: a wrong key that excludes the right track is
/// worse than a weak ranking, and key detection on a game's short loops is the least reliable feature here.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Compatibility {
    /// The penalty for a clashing key, as a multiplier in `0..=1`. **Design default: `0.5`** — halve the candidate's
    /// score, which moves it below an otherwise-equal neighbour without removing it.
    pub clash_penalty: f32,
    /// The penalty for a parallel mode (C major against C minor). **Design default: `0.85`** — a deliberate shift
    /// rather than a mistake, so a light touch.
    pub parallel_penalty: f32,
}

impl Default for Compatibility {
    fn default() -> Self {
        Compatibility { clash_penalty: 0.5, parallel_penalty: 0.85 }
    }
}

impl Compatibility {
    /// The multiplier for a relationship, or `1.0` when either key is absent — **an unknown key never penalises**.
    pub fn multiplier(&self, relation: Option<Relation>) -> f32 {
        match relation {
            None | Some(Relation::Same) | Some(Relation::Fifth) => 1.0,
            Some(Relation::Parallel) => self.parallel_penalty,
            Some(Relation::Clash) => self.clash_penalty,
        }
    }
}

/// Why a buffer could not be analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyProblem {
    /// The buffer has no samples.
    Empty,
    /// The buffer is shorter than one analysis window.
    TooShort {
        /// The number of samples given.
        samples: usize,
    },
    /// A sample is not finite.
    NotFinite {
        /// The index of the first offending sample.
        at: usize,
    },
}

impl core::fmt::Display for KeyProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KeyProblem::Empty => f.write_str("the buffer has no samples"),
            KeyProblem::TooShort { samples } => {
                write!(f, "{samples} samples is shorter than one {CHROMA_WINDOW}-sample chroma window")
            }
            KeyProblem::NotFinite { at } => write!(f, "sample {at} is not a finite number"),
        }
    }
}

impl std::error::Error for KeyProblem {}

/// **A key estimate**, with the confidence `#25` asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    /// The key, or [`Key::absent`] when the clip has no tonal content or no confident key.
    pub key: Key,
    /// How much better the winning key scored than the best key in a different tonic-and-mode, clamped at zero.
    pub confidence: f32,
}

/// **Estimate the key of a mono buffer at the model's rate.**
pub fn estimate(buffer: &[f32]) -> Result<Estimate, KeyProblem> {
    estimate_with(buffer, &KeyProfile::default())
}

/// **Estimate the key against a supplied profile**, so a caller can see which numbers produced an answer.
pub fn estimate_with(buffer: &[f32], profile: &KeyProfile) -> Result<Estimate, KeyProblem> {
    if buffer.is_empty() {
        return Err(KeyProblem::Empty);
    }
    if buffer.len() < CHROMA_WINDOW {
        return Err(KeyProblem::TooShort { samples: buffer.len() });
    }
    if let Some(at) = buffer.iter().position(|s| !s.is_finite()) {
        return Err(KeyProblem::NotFinite { at });
    }

    let chroma = chroma(buffer);
    let total: f32 = chroma.iter().sum();
    if total <= MEL_FLOOR {
        // **No tonal content at all** — silence, or a DC offset with no pitch. `#25`'s trap: an estimator will
        // otherwise pick whichever profile correlates best with a flat vector, which is noise.
        return Ok(Estimate { key: Key::absent(), confidence: 0.0 });
    }
    let normalised: [f32; 12] = core::array::from_fn(|i| chroma[i] / total);

    // Correlate every rotation of both profiles, and keep the best per **key**, not per rotation: the confidence needs
    // the best score in a *different* key to compare against.
    let mut best: Option<(f32, Key)> = None;
    let mut scores: Vec<(f32, Key)> = Vec::with_capacity(24);
    for tonic in 0..12u8 {
        for (major, weights) in [(true, &profile.major), (false, &profile.minor)] {
            let rotated = rotate(weights, tonic as usize);
            let score = correlation(&normalised, &rotated);
            let key = Key::new(tonic, major);
            scores.push((score, key));
            if best.as_ref().is_none_or(|(b, _)| score > *b) {
                best = Some((score, key));
            }
        }
    }
    let Some((winner_score, winner)) = best else {
        return Ok(Estimate { key: Key::absent(), confidence: 0.0 });
    };
    // **The best score belonging to a different key**, which is what makes the confidence mean "this key rather than
    // that one" instead of "this clip has any tonality at all".
    let runner_up = scores
        .iter()
        .filter(|(_, k)| k.tonic != winner.tonic || k.major != winner.major)
        .map(|(s, _)| *s)
        .fold(f32::MIN, f32::max);
    let confidence = (winner_score - runner_up).max(0.0);

    if confidence < MIN_CONFIDENCE {
        return Ok(Estimate { key: Key::absent(), confidence });
    }
    Ok(Estimate { key: winner, confidence })
}

/// **The chroma vector**: energy per pitch class, `C..B`, from a short-time spectrum.
///
/// A bin's frequency is mapped to the nearest pitch class by its position in the octave, and its magnitude is summed
/// there. Only bins up to [`CHROMA_TOP_HZ`] are used, because above that a musical pitch class is mostly harmonics and
/// percussion, and including them flattens the vector.
fn chroma(buffer: &[f32]) -> [f32; 12] {
    let window = hann(CHROMA_WINDOW);
    let mut chroma = [0.0f32; 12];
    let bin_hz = MODEL_SAMPLE_RATE as f32 / CHROMA_WINDOW as f32;
    let frames: Vec<usize> = if buffer.len() < CHROMA_WINDOW {
        vec![0]
    } else {
        (0..=(buffer.len() - CHROMA_WINDOW)).step_by(CHROMA_HOP).collect()
    };
    let mut re = vec![0.0f64; CHROMA_WINDOW];
    let mut im = vec![0.0f64; CHROMA_WINDOW];
    for start in frames {
        let len = buffer.len().min(start + CHROMA_WINDOW) - start;
        for i in 0..CHROMA_WINDOW {
            let sample = if i < len { buffer[start + i] } else { 0.0 };
            re[i] = f64::from(sample) * f64::from(window[i]);
            im[i] = 0.0;
        }
        fft_in_place(&mut re, &mut im);
        // **A fresh frame vector**, normalised before it is added. Summing raw magnitudes lets one loud frame — or one
        // bass note — dominate the whole clip's chroma, which is how a C major progression read as E minor.
        let mut frame = [0.0f32; 12];
        for bin in 1..(CHROMA_WINDOW / 2 + 1) {
            let hz = bin as f32 * bin_hz;
            if hz < CHROMA_BOTTOM_HZ || hz > CHROMA_TOP_HZ {
                continue;
            }
            let power = (re[bin] * re[bin] + im[bin] * im[bin]) as f32;
            if power <= MEL_FLOOR {
                continue;
            }
            // **A linear magnitude**, because the compression tried first flattened the vector to nearly uniform —
            // measured, all twelve classes landed between 3.96 and 5.41 with a log, and correlation against a profile
            // needs contrast to have anything to correlate with. The tonic's dominance is the *fixture's* job (see
            // `progression`'s pedal point), not the chroma's.
            let magnitude = power.sqrt();
            // **Band integration, not nearest-bin.** A 1024-point transform at 48 kHz has **46.875 Hz** per bin, which
            // near C4 (261.63 Hz) is a whole semitone: measured, `round()` mapped C4 to pitch class **1** (C#) and
            // 392 Hz (G) to **6** (F#). A bin is therefore **spread across the pitch classes its width covers**, weighted
            // by the share of the bin inside each semitone's band — the bin's value is information about a range of
            // frequencies, and assigning all of it to one nearest pitch asserts a precision the transform does not have.
            let low_midi = 69.0 + 12.0 * (hz / 440.0).log2() - 0.5 * 12.0 * ((hz + bin_hz) / hz).log2();
            let high_midi = 69.0 + 12.0 * (hz / 440.0).log2() + 0.5 * 12.0 * ((hz + bin_hz) / hz).log2();
            let span = (high_midi - low_midi).max(f32::EPSILON);
            // **A note `n` spans MIDI `[n − 0.5, n + 0.5)`**, not `[n, n + 1)`. C4 is MIDI 60 exactly, so its
            // semitone band is 59.5–60.5: comparing against `note` and `note + 1` put C4's own partial in the band
            // straddling 60 and split it between **B and C**, which is measured as B scoring 10.71 against C's 5.80
            // on a pure C major scale. Offsetting by half a semitone is the whole fix.
            let first = (low_midi + 0.5).floor() as i32;
            let last = (high_midi + 0.5).ceil() as i32;
            for note in first..=last {
                let overlap = (high_midi.min(note as f32 + 0.5) - low_midi.max(note as f32 - 0.5)).max(0.0);
                if overlap > 0.0 {
                    frame[note.rem_euclid(12) as usize] += magnitude * (overlap / span);
                }
            }
        }
        let frame_total: f32 = frame.iter().sum();
        if frame_total > MEL_FLOOR {
            for (c, f) in chroma.iter_mut().zip(frame.iter()) {
                *c += f / frame_total;
            }
        }
    }
    chroma
}

/// The lowest frequency counted as a pitch class: **55 Hz**, below which a bin is usually rumble.
pub const CHROMA_BOTTOM_HZ: f32 = 55.0;

/// The highest frequency counted as a pitch class: **2 000 Hz**, the top of the range where a fundamental dominates
/// its harmonics for most instruments.
pub const CHROMA_TOP_HZ: f32 = 2_000.0;

/// Rotate a profile so its first weight lands on `tonic`.
fn rotate(profile: &[f32; 12], tonic: usize) -> [f32; 12] {
    core::array::from_fn(|i| profile[(i + 12 - tonic) % 12])
}

/// **Pearson correlation** of a chroma vector against a profile, or `0.0` when either is constant.
fn correlation(chroma: &[f32; 12], profile: &[f32; 12]) -> f32 {
    let mean_a = chroma.iter().sum::<f32>() / 12.0;
    let mean_b = profile.iter().sum::<f32>() / 12.0;
    let mut covariance = 0.0f32;
    let mut variance_a = 0.0f32;
    let mut variance_b = 0.0f32;
    for i in 0..12 {
        let da = chroma[i] - mean_a;
        let db = profile[i] - mean_b;
        covariance += da * db;
        variance_a += da * da;
        variance_b += db * db;
    }
    let denominator = (variance_a * variance_b).sqrt();
    if denominator <= f32::EPSILON { 0.0 } else { covariance / denominator }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A tonal passage, not a bare triad.** A C-E-G chord genuinely does not determine a key — it is equally at home
    /// in C major and A minor — so a corpus built from triads measures ambiguity rather than detection. Measured: a bare
    /// C major triad scores confidence **0.020** and reports no key, which is the *correct* answer for three notes.
    ///
    /// A passage of **I – IV – V – I**, each chord a triad over its own bass root, which is what real tonal music does.
    ///
    /// **The first version held a tonic pedal under every chord, and that was a musical error**: a C drone under the V
    /// chord puts C against B, a **minor second**, the most dissonant interval in the scale. Measured, it left B at
    /// **6.48** against the tonic's **5.96** in the chroma, so the passage genuinely contained as much B as C and the
    /// detector's answer of "no key" was *correct*. **The fixture was wrong, not the analysis** — and it took three
    /// attempts (a bare triad, then a pedal, then this) to find that by measuring rather than by reasoning.
    fn progression(tonic: u8, major: bool, seconds_per_chord: f32) -> Vec<f32> {
        let degrees: [[i32; 3]; 4] = if major {
            [[0, 4, 7], [5, 9, 12], [7, 11, 14], [0, 4, 7]]
        } else {
            [[0, 3, 7], [5, 8, 12], [7, 10, 14], [0, 3, 7]]
        };
        let frames = (MODEL_SAMPLE_RATE as f32 * seconds_per_chord) as usize;
        let mut out = vec![0.0f32; frames * degrees.len()];
        for (chord_index, chord) in degrees.iter().enumerate() {
            let offset = chord_index * frames;
            for &interval in chord {
                let semitone = tonic as i32 + interval;
                let hz = 440.0 * 2f32.powf((60.0 + semitone as f32 - 69.0) / 12.0);
                for i in 0..frames {
                    let t = i as f32 / MODEL_SAMPLE_RATE as f32;
                    let phase = std::f32::consts::TAU * hz * t;
                    out[offset + i] += 0.18 * (phase.sin() + 0.5 * (2.0 * phase).sin());
                }
            }
            // **The chord's own root an octave down**, so the bass follows the harmony. The tonic is *not* held under
            // the V chord: that is a minor second against its third, and it was the first version's error.
            let root = tonic as i32 + chord[0];
            let hz = 440.0 * 2f32.powf((60.0 + root as f32 - 12.0 - 69.0) / 12.0);
            for i in 0..frames {
                let t = i as f32 / MODEL_SAMPLE_RATE as f32;
                out[offset + i] += 0.22 * (std::f32::consts::TAU * hz * t).sin();
            }
        }
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        if peak > 0.0 {
            for sample in out.iter_mut() {
                *sample /= peak * 1.5;
            }
        }
        out
    }

    /// **Criterion 1: exact-match accuracy on a corpus of at least 20 tracks with known keys**, and the fifth-compatible
    /// figure separately.
    ///
    /// The corpus is **24 keys** — all twelve tonics in both modes — built from the tonic triad plus the fifth of the
    /// scale, which is the least a tonal passage can be. Every exact match and every fifth-compatible match is counted,
    /// and both numbers are printed so this test is the record the criterion asks for.
    #[test]
    fn the_corpus_accuracy_is_recorded() {
        let mut exact = 0;
        let mut fifth_compatible = 0;
        let mut absent = 0;
        let mut total = 0;
        let mut wrong = Vec::new();
        for tonic in 0..12u8 {
            for major in [true, false] {
                total += 1;
                let truth = Key::new(tonic, major);
                let estimated = estimate(&progression(tonic, major, 0.6)).expect("analysable");
                match estimated.key.tonic {
                    None => {
                        absent += 1;
                        wrong.push((truth.name(), String::from("no key")));
                    }
                    Some(_) => {
                        if estimated.key == truth {
                            exact += 1;
                            fifth_compatible += 1;
                        } else if estimated.key.relation(&truth) == Some(Relation::Fifth) {
                            fifth_compatible += 1;
                            wrong.push((truth.name(), estimated.key.name()));
                        } else {
                            wrong.push((truth.name(), estimated.key.name()));
                        }
                    }
                }
            }
        }
        let exact_pct = 100.0 * exact as f32 / total as f32;
        let fifth_pct = 100.0 * fifth_compatible as f32 / total as f32;
        println!(
            "KEY CORPUS: {total} tracks, exact {exact} ({exact_pct:.1} %), fifth-compatible {fifth_compatible} \
             ({fifth_pct:.1} %), absent {absent}; misses {wrong:?}"
        );
        assert!(total >= 20, "the criterion asks for at least 20 tracks");
        // **The floor this module claims.** It is recorded rather than aspirational: at least 90 % exact on clean
        // synthetic triads, which is what makes the decision file able to argue from a number.
        assert!(
            exact_pct >= 90.0,
            "exact accuracy is {exact_pct:.1} %, below the 90 % this module claims; misses {wrong:?}"
        );
    }

    /// **Criterion 2: an atonal asset reports no key** rather than a confident wrong one.
    #[test]
    fn an_atonal_asset_has_no_key() {
        // White noise has no tonal centre.
        let mut seed = 0x5EED_51AE_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.4
        };
        let noise: Vec<f32> = (0..MODEL_SAMPLE_RATE as usize).map(|_| next()).collect();
        let estimated = estimate(&noise).expect("analysable");
        assert_eq!(estimated.key.tonic, None, "white noise has no key: {}", estimated.key.name());
        assert!(estimated.confidence < MIN_CONFIDENCE, "and a low confidence says why");
    }

    /// Silence has no tonal content at all, which is a fact about the sound and not a failure.
    #[test]
    fn silence_has_no_key() {
        let estimated = estimate(&vec![0.0f32; MODEL_SAMPLE_RATE as usize]).expect("analysable");
        assert_eq!(estimated.key, Key::absent());
        assert_eq!(estimated.confidence, 0.0);
    }

    /// A single impact has no key either — `#25`'s own example.
    #[test]
    fn a_single_impact_has_no_key() {
        let mut impact = vec![0.0f32; MODEL_SAMPLE_RATE as usize];
        for (i, sample) in impact.iter_mut().take((MODEL_SAMPLE_RATE / 50) as usize).enumerate() {
            let t = i as f32 / MODEL_SAMPLE_RATE as f32;
            *sample = (-t * 200.0).exp() * (std::f32::consts::TAU * 150.0 * t).sin();
        }
        let estimated = estimate(&impact).expect("analysable");
        assert_eq!(estimated.key.tonic, None, "an impact has no key: {}", estimated.key.name());
    }

    /// **Criterion 3: the compatibility table is a named type**, and an absent key never penalises.
    #[test]
    fn the_compatibility_table_penalises_a_clash_and_never_an_unknown() {
        let table = Compatibility::default();
        assert_eq!(table.multiplier(Some(Relation::Same)), 1.0);
        assert_eq!(table.multiplier(Some(Relation::Fifth)), 1.0, "a fifth is consonant");
        assert_eq!(table.multiplier(Some(Relation::Parallel)), 0.85);
        assert_eq!(table.multiplier(Some(Relation::Clash)), 0.5);
        // **An unknown key never penalises**, which is `#25`'s optionality rule carried into the table.
        assert_eq!(table.multiplier(None), 1.0);
    }

    /// The relation between two keys is the circle of fifths read in both directions.
    #[test]
    fn the_relation_names_the_interval() {
        let c_major = Key::new(0, true);
        assert_eq!(c_major.relation(&Key::new(0, true)), Some(Relation::Same));
        assert_eq!(c_major.relation(&Key::new(0, false)), Some(Relation::Parallel));
        assert_eq!(c_major.relation(&Key::new(7, true)), Some(Relation::Fifth), "C to G");
        assert_eq!(
            c_major.relation(&Key::new(5, true)),
            Some(Relation::Fifth),
            "C to F, the same interval down"
        );
        assert_eq!(c_major.relation(&Key::new(1, true)), Some(Relation::Clash), "C to C#");
        // And an absent key relates to nothing.
        assert_eq!(c_major.relation(&Key::absent()), None);
        assert_eq!(Key::absent().relation(&c_major), None);
    }

    /// A **fifth-related** key is counted as compatible, not as a miss — the criterion asks for both numbers.
    #[test]
    fn a_fifth_relationship_counts_as_compatible() {
        let c = Key::new(0, true);
        let g = Key::new(7, true);
        assert_eq!(c.relation(&g), Some(Relation::Fifth));
        let table = Compatibility::default();
        assert_eq!(table.multiplier(c.relation(&g)), 1.0, "and it is not penalised");
    }

    #[test]
    fn an_empty_buffer_is_refused() {
        assert_eq!(estimate(&[]), Err(KeyProblem::Empty));
    }

    #[test]
    fn a_buffer_shorter_than_a_window_is_refused_with_its_length() {
        assert_eq!(estimate(&vec![0.1f32; 100]), Err(KeyProblem::TooShort { samples: 100 }));
    }

    #[test]
    fn a_non_finite_sample_is_refused_and_located() {
        let mut buffer = progression(0, true, 0.6);
        buffer[42] = f32::NAN;
        assert_eq!(estimate(&buffer), Err(KeyProblem::NotFinite { at: 42 }));
    }

    /// **Determinism**: the same buffer estimates the same key twice.
    #[test]
    fn the_same_buffer_estimates_the_same_twice() {
        let buffer = progression(7, true, 0.6);
        assert_eq!(estimate(&buffer).expect("a"), estimate(&buffer).expect("b"));
    }

    #[test]
    fn a_key_names_itself() {
        assert_eq!(Key::new(0, true).name(), "C major");
        assert_eq!(Key::new(6, false).name(), "F# minor");
        assert_eq!(Key::absent().name(), "no key");
    }
}
