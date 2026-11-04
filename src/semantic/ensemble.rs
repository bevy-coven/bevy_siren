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
//! decision."* Every signal is therefore an **agreement in `[-1, 1]`**, where `+1` is "as alike as the signal can say",
//! `0` is "neither", and `-1` is "as far apart as the rule already considers fatal":
//!
//! | signal | raw value | why that number |
//! |---|---|---|
//! | semantic | the cosine, unchanged | it **is** already `[-1, 1]`, so no map can improve it |
//! | acoustic | `1 − 2·difference / tolerance`, clamped | `+1` at equality, `0` at half the tolerance, `−1` at the tolerance |
//! | tempo | the same shape, on `tempo_tolerance_bpm` | |
//! | key | `2·multiplier − 1` | the table's `1.0` becomes `+1`, its `0.5` becomes `0`, its `0.0` becomes `−1` |
//!
//! ## Why the scale is signed and not `[0, 1]`
//!
//! A first version mapped everything into `[0, 1]` and was **wrong in a way only a test caught**: every contribution
//! then had to be positive, so [`Breakdown::strongest`] — which looks for a contribution **below zero** to report what
//! counted *against* a result — could never find one. A breakdown whose `explain()` can only ever name a positive is
//! not a breakdown; it is a score with decoration.
//!
//! The signed scale also keeps the semantic signal honest: a cosine of `−0.2` means the two sounds are *unrelated*,
//! and reporting that as `0.4` of agreement would call an antipathy a mild match.
//!
//! ## Why not a squashing function
//!
//! A sigmoid or a power would change the *order* of results once other signals are added, and nothing measured says the semantic
//! score is mis-scaled: on the golden set the top result is right for 72 – 90 % of descriptions (a stranger's wording to the
//! maintainer's) and the weakest top result scores 0.41 – 0.49, so there is no evidence a transform would sharpen anything worth
//! sharpening. Both maps here are affine, so they change units only.
//!
//! # A missing signal is not a disagreement
//!
//! Every raw value here is `Option`. An asset with no tempo estimated, or a key that was judged atonal, contributes
//! nothing and is marked [`Contribution::missing`] — never a zero, which would read as *"consulted and disagreed"*. That
//! distinction is the one [`Breakdown`]'s own doc calls out, and it is what makes [`Breakdown::explain`] honest.

use crate::model::{Breakdown, Contribution, Signal};

use super::filter::{Exclusion, Judgement};
use super::weights::Weights;

/// **The agreement one difference represents**, on the signed scale.
///
/// `1 − 2·difference / tolerance`: `+1` when the two are identical, `0` at **half** the tolerance, `−1` at the
/// tolerance, and clamped beyond it. Half is the midpoint because the tolerance is the point the rule calls fatal, so
/// it has to map to the bottom of the scale rather than the middle — otherwise a pair sitting exactly at the threshold
/// would read as "neutral" and the breakdown would call it neither help nor harm when the filter had just excluded it.
fn within(difference: f32, tolerance: f32) -> f32 {
    if !difference.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return -1.0;
    }
    (1.0 - 2.0 * difference.abs() / tolerance).clamp(-1.0, 1.0)
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
    // **Semantic**: the cosine itself, which is already on the scale. `None` for a non-finite one rather than a NaN
    // weight — `Contribution::scored` would mark it missing anyway, but saying it here keeps the array honest.
    let semantic = if cosine.is_finite() { Some(cosine.clamp(-1.0, 1.0)) } else { None };

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

    // **Key**: the table's multiplier, mapped from `[0, 1]` onto `[-1, 1]` so `1.0` is full agreement and `0.5` is
    // neutral rather than a positive contribution. A project that sets `clash_penalty: 1.0` gets `+1`, which is
    // exactly the "this rule stops penalising" the guide promises.
    let key = judgement.penalties.iter().find_map(|p| match p {
        Exclusion::Key { multiplier, .. } => Some(2.0 * multiplier.clamp(0.0, 1.0) - 1.0),
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
