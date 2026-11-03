//! **Every signal on one scale, and the sum that ranks with it** — `#40`'s scoring seam.
//!
//! [`Breakdown`] in `src/model.rs` owns the arithmetic: a contribution is `raw * weight`, the score is their sum, and the
//! shares normalise it. It sits there because `tests/all.rs` scans that file for engine types, so the arithmetic cannot
//! quietly grow a `Res<…>` and need an app to test. That leaves one gap, and this module is it: nothing built a
//! [`Breakdown`] from the signals the crate actually computes.
//!
//! # The scale, which is the authored decision
//!
//! `#40`'s own pitfall: *"scores from different components are not on the same scale. A raw cosine in [-1, 1] and a
//! loudness penalty in LU must be mapped onto a common range before weighting, and that mapping is itself an authored
//! decision."* Every signal here is therefore an **agreement in `[0, 1]`**, where `1` means "as alike as the rule allows"
//! and `0` means "at or beyond the threshold the rule already uses":
//!
//! | signal | raw value | why that number |
//! |---|---|---|
//! | semantic | `(cosine + 1) / 2` | cosine is in `[-1, 1]`; the map is affine, so it **preserves order exactly** |
//! | acoustic | `1 − difference / tolerance`, clamped to `[0, 1]` | reuses the tolerance `filter::evaluate` already refuses on, so the two cannot disagree about where "too far" is |
//! | tempo | `1 − difference / tolerance`, clamped | the same shape, on `tempo_tolerance_bpm` |
//! | key | `Compatibility::multiplier` | already `[0, 1]` and already the authored table — no second mapping |
//!
//! The affine map on cosine is the part worth arguing: a squashing function (a sigmoid, say) would change the *order* of
//! results, and this crate's measured semantic band overlaps the misses (0.126–0.429 against −0.104–0.150), so a
//! transform that sharpens the top would sharpen the wrong thing. `(c + 1) / 2` keeps `top_k`'s order intact and only
//! changes the units.
//!
//! # A missing signal is not a disagreement
//!
//! Every raw value here is `Option`. An asset with no tempo estimated, or a key that was judged atonal, contributes
//! nothing and is marked [`Contribution::missing`] — never a zero, which would read as *"consulted and disagreed"*. That
//! distinction is the one [`Breakdown`]'s own doc calls out, and it is what makes [`Breakdown::explain`] honest.

use crate::model::{Breakdown, Contribution, Signal};

use super::filter::{Exclusion, Judgement};
use super::weights::Weights;

/// **The agreement one penalty represents**, as a number in `[0, 1]`.
///
/// `1 − difference / tolerance` is the same fraction the rule thresholded on, read backwards: at the tolerance the
/// agreement is `0`, at no difference it is `1`, and beyond the tolerance it stays `0` because a signal that already
/// failed its hard rule cannot be *more* failed.
fn within(difference: f32, tolerance: f32) -> f32 {
    if !difference.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return 0.0;
    }
    (1.0 - difference / tolerance).clamp(0.0, 1.0)
}

/// **Every signal's raw agreement for one candidate**, before any weight is applied.
///
/// Returned as an array indexed by [`Signal::index`], each value `None` when that signal had nothing to say. A caller that
/// only wants the score can pass this straight to [`breakdown`]; a caller that wants to log the raw numbers can print it.
pub fn agreements(
    cosine: f32,
    candidate: &super::filter::Features,
    anchor: &super::filter::Features,
    judgement: &Judgement,
    weights: &Weights,
) -> [Option<f32>; 4] {
    // **Semantic**: the affine map, which preserves order. `None` for a non-finite cosine rather than a NaN weight —
    // `Contribution::scored` would mark it missing anyway, but saying it here keeps the array honest.
    let semantic = if cosine.is_finite() { Some(((cosine + 1.0) / 2.0).clamp(0.0, 1.0)) } else { None };

    // **Acoustic**: the noisier of the two penalties the filter already found. It is read from the judgement rather than
    // recomputed, so this can never disagree with the rule that excluded the candidate.
    let acoustic = judgement
        .penalties
        .iter()
        .find_map(|p| match p {
            Exclusion::NoiseFloor { difference_db, tolerance_db } => {
                Some(within(*difference_db, *tolerance_db))
            }
            _ => None,
        })
        .or_else(|| {
            // No noise-floor penalty means the pair is inside the tolerance — but only if BOTH sides measured one.
            // An unmeasured pair is `None`, not a perfect `1.0`, which would reward an asset for being unanalysable.
            match (anchor.noise_floor_db(), candidate.noise_floor_db()) {
                (Some(a), Some(c)) => Some(within((c - a).abs(), weights.noise_floor_tolerance_db)),
                _ => None,
            }
        });

    // **Tempo**: from the pair's own numbers, because a pair inside the tolerance produces no penalty to read.
    let tempo = match (anchor.bpm(), candidate.bpm()) {
        (Some(a), Some(c)) => Some(within((c - a).abs(), weights.tempo_tolerance_bpm)),
        _ => None,
    };

    // **Key**: the compatibility table's own multiplier, which is already an agreement in `[0, 1]`.
    let key = judgement.penalties.iter().find_map(|p| match p {
        Exclusion::Key { multiplier, .. } => Some(multiplier.clamp(0.0, 1.0)),
        _ => None,
    });

    [semantic, acoustic, tempo, key]
}

/// **The weighted breakdown for one candidate**, which is what a result carries so a caller can say why it ranked.
pub fn breakdown(
    cosine: f32,
    candidate: &super::filter::Features,
    anchor: &super::filter::Features,
    judgement: &Judgement,
    weights: &Weights,
) -> Breakdown {
    let raw = agreements(cosine, candidate, anchor, judgement, weights);
    let weight_of = |s: Signal| match s {
        Signal::Semantic => weights.semantic,
        Signal::Acoustic => weights.acoustic,
        Signal::Tempo => weights.tempo,
        Signal::Key => weights.key,
    };
    let mut parts = [
        Contribution::absent(Signal::Semantic, weights.semantic),
        Contribution::absent(Signal::Acoustic, weights.acoustic),
        Contribution::absent(Signal::Tempo, weights.tempo),
        Contribution::absent(Signal::Key, weights.key),
    ];
    for s in crate::model::SIGNALS {
        parts[s.index()] = match raw[s.index()] {
            Some(value) => Contribution::scored(s, value, weight_of(s)),
            None => Contribution::absent(s, weight_of(s)),
        };
    }
    Breakdown::new(parts)
}
