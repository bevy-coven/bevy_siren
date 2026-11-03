//! **How reverberant a recording is**, as an early-to-late energy ratio — `#23`.
//!
//! A dry studio effect against a wet cave effect is a world mismatch an embedding blurs, because both are *"dripping
//! water"*. This is the feature that separates them, and `#23` is candid that it is *"the feature most likely to be a
//! number that looks precise and means little"* — so the method, its range and its failure modes are stated here rather
//! than implied by a field name.
//!
//! # The measure
//!
//! **`C50`, ISO 3382's clarity index**, adapted to a single channel with no impulse response:
//!
//! ```text
//! C50 = 10 · log₁₀ ( energy in the 50 ms after the peak  /  energy after that 50 ms )   [dB]
//! ```
//!
//! ISO 3382 defines C50 for a measured room impulse response, where the direct sound's arrival is known because the
//! excitation is. A game asset is not an impulse response, so this uses the **loudest frame as the direct-sound proxy**
//! and calls the 50 ms after it *early* and everything beyond *late*. That is an approximation, and it is the honest one
//! available without a model: an alternative — training a masking network, as Mack, Deng & Habets do for speech
//! (`10.21437/interspeech.2020-2171`) — is exactly the ML dependency this crate's acoustic features exist to avoid.
//!
//! # What the number means
//!
//! | C50 | reads as |
//! |---|---|
//! | **+20 dB and above** | dry: nearly all the energy is in the first 50 ms |
//! | **+10 to +20 dB** | a small room, or a short plate |
//! | **0 to +10 dB** | an audible room |
//! | **below 0 dB** | wet: more energy after the 50 ms than in it — a cave, a cathedral |
//!
//! The range is **−∞ to +∞ in dB** and in practice **−30 to +40**. The upper end is unbounded for a mathematical reason
//! worth stating: a digital-silence tail makes the denominator zero, and that is reported as [`Reverb::CEILING_DB`] rather than
//! as an infinite number — an infinity in a filter comparison compares false against everything and would silently
//! remove the asset from every ranking.
//!
//! # The failure modes, stated
//!
//! - **A clip too short to have a late window** — shorter than `early_window_s` after its peak — has **no measure**, and
//!   reports `None`. A 0.1 s footstep is exactly this case: it is dry by construction, and a `+40 dB` would be a
//!   confident wrong answer about a clip that never had room to be wet.
//! - **A clip whose peak is at the very end** has no late window either, for the same reason.
//! - **A sustained pad** with no decay has a peak somewhere mid-clip and a large "late" energy that is really the
//!   sustain, so it reads wetter than it is. That is a real limitation of an envelope measure on non-percussive material,
//!   and it is why this signal is **not** in the default weights until its discrimination is measured — see
//!   [`DISCRIMINATION_MEASURED`].

use super::analysis::spectra;

/// **The windows the estimate is taken over**, on a serde type because a rate and a threshold both are.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct ReverbSettings {
    /// The boundary between *early* and *late*, in seconds.
    ///
    /// **Default: 0.05**, which is **ISO 3382's C50**: fifty milliseconds is where a reflection stops fusing with the
    /// direct sound perceptually and starts being heard as room.
    pub early_window_s: f32,
    /// How far past the early window the *late* energy is accumulated, in seconds.
    ///
    /// **Default: 0.5** — **design default**. Long enough that a decaying tail is inside it (a 0.5 s window sees an
    /// RT60 of up to about 0.4 s before truncating), short enough that a long quiet passage after a sound does not count
    /// as its reverb.
    pub late_window_s: f32,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        ReverbSettings { early_window_s: 0.05, late_window_s: 0.5 }
    }
}

/// **The discrimination this measure was recorded to have**, on the labelled dry/wet pairs in this module's tests.
///
/// `#23` asks for it to be *"recorded as a number"* and to be candid if it is poor. Measured: see
/// [`discrimination`] and the tests — the dry and wet estimates separate by a wide margin with **no overlap** on the
/// synthetic corpus, but that corpus is synthetic. Real recordings of a cave and a studio have not been measured, and
/// until they are this constant says so and the signal stays out of the default weights.
pub const DISCRIMINATION_MEASURED: &str = "synthetic only: 12 dry/wet pairs, every pair separated in the expected \
    direction with no overlap; real recordings not measured";

/// **How reverberant a clip is**, by ISO 3382's C50 read from an envelope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reverb {
    /// The early-to-late ratio in dB, or `None` when the clip has no late window to measure.
    ///
    /// `Some(f32::INFINITY)` is never produced: a zero late denominator is reported as a large but finite value (see
    /// [`Reverb::CEILING_DB`]) because an infinity compares false against every threshold.
    pub early_to_late_db: Option<f32>,
    /// Where the direct-sound proxy was found, in seconds, so a caller can see what the measure latched onto.
    pub peak_at_s: f32,
}

impl Reverb {
    /// **The reported ceiling for a clip with no late energy at all.**
    ///
    /// **Design default: `+60 dB`** — 1000:1 in energy, past which the distinction stops being audible and any further
    /// number is arithmetic about a zero rather than a measurement. Reported instead of infinity so a filter comparison
    /// is meaningful.
    pub const CEILING_DB: f32 = 60.0;

    /// A clip with no measurable tail: dry by construction rather than by measurement.
    pub fn unmeasured(peak_at_s: f32) -> Self {
        Reverb { early_to_late_db: None, peak_at_s }
    }

    /// The ratio, clamped to [`Reverb::CEILING_DB`], or `None`.
    pub fn db(&self) -> Option<f32> {
        self.early_to_late_db
    }

    /// True when the measure exists (a clip long enough to have a late window).
    pub fn measured(&self) -> bool {
        self.early_to_late_db.is_some()
    }
}

/// **Estimate the reverberance of a mono buffer at the model's rate.**
///
/// The frames come from [`spectra`], so this costs no second transform — and it is the same envelope tempo reads, which
/// is why the estimate is cheap.
///
/// Returns `None` for `early_to_late_db` when the clip has no late window: shorter than `early_window_s` after its peak,
/// or empty. `peak_at_s` is still reported, because where the loudest moment was is a fact even when the tail cannot be
/// measured.
pub fn estimate(buffer: &[f32], settings: ReverbSettings) -> Reverb {
    if buffer.is_empty() || settings.early_window_s <= 0.0 || settings.late_window_s <= 0.0 {
        return Reverb::unmeasured(0.0);
    }
    let rate = super::MODEL_SAMPLE_RATE as f32;
    let hop_s = super::mel::HOP as f32 / rate;

    // **Frame energies, from the shared pass.** One value per frame; a frame is `HOP` samples.
    let frames = spectra(buffer).frame_energy;

    let Some((peak_frame, _)) =
        frames.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
    else {
        return Reverb::unmeasured(0.0);
    };
    let peak_at_s = peak_frame as f32 * hop_s;

    let early_frames = (settings.early_window_s / hop_s).round().max(1.0) as usize;
    let late_frames = (settings.late_window_s / hop_s).round().max(1.0) as usize;

    let early: f64 = frames.iter().skip(peak_frame).take(early_frames).map(|e| f64::from(*e)).sum();
    let late: f64 =
        frames.iter().skip(peak_frame + early_frames).take(late_frames).map(|e| f64::from(*e)).sum();

    // **No late window at all** — the clip ends before the boundary. Not a dry measurement, an absent one.
    if frames.len() <= peak_frame + early_frames {
        return Reverb::unmeasured(peak_at_s);
    }
    if early <= 0.0 {
        return Reverb::unmeasured(peak_at_s);
    }
    if late <= 0.0 {
        // A tail of exact digital silence: dry past the point the measure resolves.
        return Reverb { early_to_late_db: Some(Reverb::CEILING_DB), peak_at_s };
    }
    let db = (10.0 * (early / late).log10()) as f32;
    Reverb { early_to_late_db: Some(db.clamp(-Reverb::CEILING_DB, Reverb::CEILING_DB)), peak_at_s }
}

/// **One dry/wet pair's estimates**, for the discrimination record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    /// The dry clip's estimate.
    pub dry: Option<f32>,
    /// The same clip with a synthetic reverb tail.
    pub wet: Option<f32>,
}

impl Pair {
    /// True when the wet copy reads **wetter**, which is the direction the issue names.
    pub fn separates(&self) -> bool {
        match (self.dry, self.wet) {
            (Some(d), Some(w)) => w < d,
            _ => false,
        }
    }
}

/// **The discrimination across a set of pairs**: how many separated, and the two ranges.
///
/// `#23`'s third criterion: *"its discrimination on the labelled dry/wet pairs is recorded as a number"*. Overlap
/// between the dry and wet ranges is the number that matters — a measure whose bands overlap cannot separate the two,
/// which is the same failure the semantic floor has and the same reason it defaults to keeping everything.
#[derive(Debug, Clone, PartialEq)]
pub struct Discrimination {
    /// Pairs that separated in the expected direction.
    pub separated: usize,
    /// Pairs offered.
    pub pairs: usize,
    /// The driest estimate seen across all dry clips.
    pub dry_min: f32,
    /// The wettest estimate seen across all wet clips.
    pub wet_max: f32,
}

impl Discrimination {
    /// The share that separated.
    pub fn rate(&self) -> f32 {
        if self.pairs == 0 { 0.0 } else { self.separated as f32 / self.pairs as f32 }
    }

    /// **True when no wet estimate is above any dry one** — the two bands are disjoint, so the measure separates them
    /// without a threshold chosen by hand.
    pub fn disjoint(&self) -> bool {
        self.wet_max < self.dry_min
    }

    /// A sentence a log shows.
    pub fn describe(&self) -> String {
        format!(
            "{}/{} pairs separated ({} %); dry ≥ {:.1} dB, wet ≤ {:.1} dB; {}",
            self.separated,
            self.pairs,
            (self.rate() * 100.0).round(),
            self.dry_min,
            self.wet_max,
            if self.disjoint() { "bands disjoint" } else { "BANDS OVERLAP" }
        )
    }
}

/// **Measure the discrimination over a set of pairs.**
pub fn discrimination(pairs: &[Pair]) -> Discrimination {
    let separated = pairs.iter().filter(|p| p.separates()).count();
    let dry_min = pairs.iter().filter_map(|p| p.dry).fold(f32::INFINITY, f32::min);
    let wet_max = pairs.iter().filter_map(|p| p.wet).fold(f32::NEG_INFINITY, f32::max);
    Discrimination {
        separated,
        pairs: pairs.len(),
        dry_min: if dry_min.is_finite() { dry_min } else { 0.0 },
        wet_max: if wet_max.is_finite() { wet_max } else { 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// A percussive dry clip: a short noise burst that stops dead.
    fn dry(seed: u64, len_s: f32) -> Vec<f32> {
        let n = (RATE * len_s) as usize;
        let mut state = seed | 1;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let noise = (state >> 40) as f32 / 8_388_608.0 - 1.0;
            // 20 ms of sound, then silence: an impulse-like direct sound.
            let envelope = if (i as f32) < RATE * 0.02 { 1.0 - (i as f32) / (RATE * 0.02) } else { 0.0 };
            out.push(noise * envelope * 0.5);
        }
        out
    }

    /// **The same clip with a synthetic reverb tail**: an exponentially decaying noise, which is the standard synthetic
    /// room impulse response and exactly the "artificially reverberated copy" the issue names.
    fn reverberate(clip: &[f32], seed: u64, rt60_s: f32) -> Vec<f32> {
        let mut state = seed.wrapping_mul(2_654_435_761) | 1;
        let decay = (-6.907_755 / (RATE * rt60_s)).exp(); // ln(1000) ≈ 6.9078, so RT60 is 60 dB of decay
        let mut level = 1.0f32;
        clip.iter()
            .map(|s| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let noise = (state >> 40) as f32 / 8_388_608.0 - 1.0;
                let wet = s + noise * level * 0.25;
                level *= decay;
                wet
            })
            .collect()
    }

    /// **Criterion 1: a dry recording and its reverberated copy separate in the expected direction.**
    #[test]
    fn a_reverberated_copy_reads_wetter_than_its_dry_source() {
        let settings = ReverbSettings::default();
        let dry_clip = dry(7, 1.5);
        let wet_clip = reverberate(&dry_clip, 11, 1.2);

        let dry_estimate =
            estimate(&dry_clip, settings).early_to_late_db.expect("a dry clip has a late window");
        let wet_estimate = estimate(&wet_clip, settings).early_to_late_db.expect("a wet clip has one too");

        assert!(
            wet_estimate < dry_estimate,
            "the wet copy must read wetter: dry {dry_estimate:.1} dB, wet {wet_estimate:.1} dB"
        );
        // And the gap is large rather than a rounding difference.
        assert!(dry_estimate - wet_estimate > 6.0, "the separation is {:.1} dB", dry_estimate - wet_estimate);
    }

    /// **Criterion 3: discrimination on a set of dry/wet pairs, recorded as a number and with no overlap.**
    #[test]
    fn the_discrimination_is_recorded_over_a_labelled_set() {
        let settings = ReverbSettings::default();
        let mut pairs = Vec::new();
        for seed in 0..12u64 {
            let dry_clip = dry(seed * 31 + 3, 1.5);
            let wet_clip = reverberate(&dry_clip, seed * 17 + 5, 0.9 + (seed as f32) * 0.05);
            pairs.push(Pair {
                dry: estimate(&dry_clip, settings).early_to_late_db,
                wet: estimate(&wet_clip, settings).early_to_late_db,
            });
        }
        let record = discrimination(&pairs);
        println!("REVERB DISCRIMINATION: {}", record.describe());

        assert_eq!(record.pairs, 12);
        assert_eq!(record.separated, 12, "every pair must separate: {}", record.describe());
        assert!(record.disjoint(), "the bands must not overlap: {}", record.describe());
    }

    /// **Criterion 2: the range and the failure mode for a fully dry clip.**
    #[test]
    fn a_fully_dry_clip_reports_the_ceiling_rather_than_an_infinity() {
        let settings = ReverbSettings::default();
        // A burst, then **exact digital silence**: no late energy at all.
        let mut clip = dry(3, 1.0);
        let boundary = (48_000.0 * 0.2) as usize;
        for s in clip.iter_mut().skip(boundary) {
            *s = 0.0;
        }
        let estimate = estimate(&clip, settings);
        let db = estimate.early_to_late_db.expect("a late window exists, and it is silent");
        assert_eq!(db, Reverb::CEILING_DB, "a zero denominator reports a finite ceiling");
        assert!(db.is_finite(), "never an infinity: it would compare false against every threshold");
    }

    /// A clip too short to have a late window has **no measure**, which is different from being dry.
    #[test]
    fn a_short_clip_has_no_measure_rather_than_a_confident_one() {
        let settings = ReverbSettings::default();
        // 40 ms is shorter than the 50 ms early window, so there is nothing beyond the boundary.
        let clip = dry(9, 0.04);
        let estimate = estimate(&clip, settings);
        assert_eq!(estimate.early_to_late_db, None, "too short to have a tail");
        assert!(!estimate.measured());
    }

    /// An empty buffer has no peak to find, and does not panic.
    #[test]
    fn an_empty_buffer_is_unmeasured() {
        let estimate = estimate(&[], ReverbSettings::default());
        assert_eq!(estimate.early_to_late_db, None);
        assert_eq!(estimate.peak_at_s, 0.0);
    }

    /// Determinism: the same buffer estimates the same twice.
    #[test]
    fn the_same_buffer_estimates_the_same_twice() {
        let clip = dry(5, 1.0);
        assert_eq!(estimate(&clip, ReverbSettings::default()), estimate(&clip, ReverbSettings::default()));
    }

    /// The default boundary is ISO 3382's C50.
    #[test]
    fn the_default_boundary_is_fifty_milliseconds() {
        assert_eq!(ReverbSettings::default().early_window_s, 0.05);
    }
}
