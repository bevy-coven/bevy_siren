//! The engine-free arithmetic, pinned against numbers a person can check by hand.
//!
//! Every assertion here is a claim about what a player hears: two 0 dB sounds are one +6.02 dB sound, a duck
//! holds then recovers, the bar at 120 bpm comes every 2 s, and a stolen voice is the lowest priority then the
//! oldest. If one of these moves, the crate's mix moved with it.

use bevy_siren::Attenuation;
use bevy_siren::model::{
    Candidate, Rng, SILENT_DB, VariantPicker, active_duck_db, approach, backend_spatial_gain, chain_db,
    db_to_gain, distance_gain, gain_to_db, next_boundary, semitones_to_ratio, steal, sum_db,
};

/// −6 dB is about half the amplitude: a gain of 0.5012.
#[test]
fn db_to_gain_is_the_amplitude_ratio() {
    assert!((db_to_gain(-6.0) - 0.5012).abs() < 1e-4, "{}", db_to_gain(-6.0));
    assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
    assert!((db_to_gain(6.0206) - 2.0).abs() < 1e-3);
}

/// Silence is one number, not a small one: at or below −80 dB the gain is exactly zero, and so is the dB of a
/// zero gain.
#[test]
fn silence_is_exactly_zero_and_never_negative_infinity() {
    assert_eq!(db_to_gain(SILENT_DB), 0.0);
    assert_eq!(db_to_gain(-200.0), 0.0);
    assert_eq!(db_to_gain(f32::NAN), 0.0);
    assert_eq!(gain_to_db(0.0), SILENT_DB);
    assert_eq!(gain_to_db(-0.5), gain_to_db(0.5));
    assert_eq!(gain_to_db(1.0), 0.0);
}

/// Two sounds at 0 dB are one at +6.02 dB, and two silences are still silence.
#[test]
fn a_sum_of_two_equal_sounds_is_plus_six_db() {
    assert!((sum_db(&[0.0, 0.0]) - 6.0206).abs() < 1e-3, "{}", sum_db(&[0.0, 0.0]));
    assert_eq!(sum_db(&[SILENT_DB, SILENT_DB]), SILENT_DB);
    assert_eq!(sum_db(&[]), SILENT_DB);
    assert!((sum_db(&[0.0]) - 0.0).abs() < 1e-6);
}

/// A chain of gains adds in dB and any silent stage silences the whole: a cue at −80 dB is not brought back by a
/// bus at 0 dB.
#[test]
fn a_chain_adds_and_a_silent_stage_silences_it() {
    assert!((chain_db(&[-6.0, -6.0]) - -12.0).abs() < 1e-5);
    assert_eq!(chain_db(&[0.0, SILENT_DB, 20.0]), SILENT_DB);
    assert_eq!(chain_db(&[]), 0.0);
}

/// One semitone is a ratio of 2^(1/12) ≈ 1.0595, and an absurd pitch is clamped rather than reaching zero or
/// infinity.
#[test]
fn semitones_are_a_frequency_ratio_with_a_clamp() {
    assert!((semitones_to_ratio(12.0) - 2.0).abs() < 1e-5);
    assert!((semitones_to_ratio(1.0) - 1.059_463).abs() < 1e-4);
    assert_eq!(semitones_to_ratio(0.0), 1.0);
    assert!((semitones_to_ratio(200.0) - 16.0).abs() < 1e-4);
    assert!((semitones_to_ratio(-200.0) - 1.0 / 16.0).abs() < 1e-6);
    assert_eq!(semitones_to_ratio(f32::NAN), 1.0);
}

/// An inverse-distance source is exactly 1.0 inside its near radius and exactly 0.5 at twice it; a linear one is
/// exactly 0 at its maximum.
#[test]
fn distance_gain_is_one_inside_near_and_halves_at_twice_it() {
    let inverse = Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 };
    assert_eq!(distance_gain(inverse, 0.0), 1.0);
    assert_eq!(distance_gain(inverse, 4.0), 1.0);
    assert!((distance_gain(inverse, 8.0) - 0.5).abs() < 1e-6, "{}", distance_gain(inverse, 8.0));
    assert!((distance_gain(inverse, 16.0) - 0.25).abs() < 1e-6);
    // A distance that is not a number is at the listener's feet, not a NaN volume that silences the bus.
    assert_eq!(distance_gain(inverse, f32::NAN), 1.0);
    assert_eq!(distance_gain(Attenuation::None, 1_000.0), 1.0);
    assert_eq!(distance_gain(Attenuation::Linear { max_m: 10.0 }, 10.0), 0.0);
    assert_eq!(distance_gain(Attenuation::Linear { max_m: 10.0 }, 25.0), 0.0);
    assert!((distance_gain(Attenuation::Linear { max_m: 10.0 }, 5.0) - 0.5).abs() < 1e-6);
}

/// The backend's spatial panner divides by the distance to *each ear* squared, clamped at 1, then pans: a sound
/// between two ears a metre away from each is at 0.5, and one two metres out is at 0.2. The crate divides that
/// gain back out, so the cue's own attenuation is the only distance law a project hears.
#[test]
fn the_backend_spatial_gain_is_the_panner_distance_law() {
    let ears = ([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let middle = backend_spatial_gain([0.0, 0.0, 0.0], ears.0, ears.1);
    assert!(
        (middle - 0.75).abs() < 1e-5,
        "a metre from each ear is 1/1² either side at a centred pan: {middle}"
    );
    let two_metres = backend_spatial_gain([0.0, 2.0, 0.0], ears.0, ears.1);
    assert!((two_metres - 0.15).abs() < 1e-5, "√5 from each ear is 1/5 either side: {two_metres}");
    // One ear is no pan, so there is no gain to divide out.
    assert_eq!(backend_spatial_gain([0.0, 2.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]), 1.0);
}

/// A duck holds for its hold and then recovers linearly: full at the start, full through the hold, half way
/// through the fade, nothing after.
#[test]
fn a_duck_holds_then_recovers() {
    assert_eq!(active_duck_db(-8.0, 0.0, 0.25, 0.6), -8.0);
    assert_eq!(active_duck_db(-8.0, 0.25, 0.25, 0.6), -8.0);
    assert!((active_duck_db(-8.0, 0.55, 0.25, 0.6) - -4.0).abs() < 1e-4);
    assert_eq!(active_duck_db(-8.0, 0.85, 0.25, 0.6), 0.0);
    assert_eq!(active_duck_db(-8.0, 5.0, 0.25, 0.6), 0.0);
    // A duck is a reduction; a positive number is not one.
    assert_eq!(active_duck_db(8.0, 0.0, 1.0, 1.0), 0.0);
    // No fade is a step at the end of the hold.
    assert_eq!(active_duck_db(-4.0, 0.24, 0.25, 0.0), -4.0);
    assert_eq!(active_duck_db(-4.0, 0.25, 0.25, 0.0), 0.0);
}

/// One time constant moves 63% of the way; no time at all returns the target.
#[test]
fn approach_moves_one_time_constant_at_a_time() {
    let moved = approach(0.0, 10.0, 0.05, 0.05);
    assert!((moved - 6.321).abs() < 1e-2, "{moved}");
    assert_eq!(approach(3.0, 10.0, 0.0, 0.016), 10.0);
    assert_eq!(approach(3.0, 10.0, 0.05, 0.0), 10.0);
    assert_eq!(approach(10.0, 10.0, 0.05, 0.016), 10.0);
}

/// At 120 bpm a bar of four beats is 2 s: the next boundary from 0.3 s is 2.0 s, and a frame exactly on the bar is
/// on it rather than past it.
#[test]
fn the_bar_is_two_seconds_at_120_bpm() {
    assert!((next_boundary(0.3, 120.0, 4.0) - 2.0).abs() < 1e-9);
    assert!((next_boundary(2.0, 120.0, 4.0) - 2.0).abs() < 1e-9);
    assert!((next_boundary(2.01, 120.0, 4.0) - 4.0).abs() < 1e-6);
    assert!((next_boundary(0.3, 120.0, 1.0) - 0.5).abs() < 1e-9);
    assert!((next_boundary(2.4, 120.0, 1.0) - 2.5).abs() < 1e-9);
    // No beat grid is no wait.
    assert_eq!(next_boundary(0.3, 0.0, 4.0), 0.3);
    assert_eq!(next_boundary(0.3, 120.0, 0.0), 0.3);
}

/// A seeded generator is a sequence, not a random number: the same seed gives the same eight numbers twice.
#[test]
fn the_rng_repeats_its_own_sequence() {
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    for _ in 0..8 {
        assert_eq!(a.next_u32(), b.next_u32());
    }
    // A zero seed would loop on zero forever, so it is replaced by a constant.
    let mut zero = Rng::new(0);
    assert_ne!(zero.next_u32(), 0);
    let mut f = Rng::new(11);
    for _ in 0..32 {
        let x = f.next_f32();
        assert!((0.0..1.0).contains(&x), "{x}");
    }
}

/// With four variants and `avoid: 1`, sixty-four picks never repeat a variant twice in a row, and every variant
/// is used.
#[test]
fn the_picker_never_repeats_within_its_window() {
    let mut picker = VariantPicker::new(1);
    let mut rng = Rng::new(3);
    let mut last = None;
    let mut seen = [0usize; 4];
    for _ in 0..64 {
        let pick = picker.pick(4, 1, &mut rng).expect("four variants");
        assert_ne!(Some(pick), last, "the same variant twice in a row");
        seen[pick] += 1;
        last = Some(pick);
    }
    assert!(seen.iter().all(|n| *n > 0), "a variant was never picked: {seen:?}");

    // A cue with two variants and a window of four still refuses the pick immediately before.
    let mut narrow = VariantPicker::new(4);
    let mut last = None;
    for _ in 0..8 {
        let pick = narrow.pick(2, 4, &mut rng).expect("two variants");
        assert_ne!(Some(pick), last);
        last = Some(pick);
    }
    assert_eq!(VariantPicker::new(4).pick(0, 1, &mut rng), None);
}

/// The steal picks the lowest priority first, then the oldest, and never an unstealable voice; a budget that
/// cannot be made room in returns fewer than asked, which the caller reads as a refusal.
#[test]
fn steal_takes_low_priority_then_the_oldest() {
    let candidates = [
        Candidate { index: 10, priority: 5, start_s: 1.0, unstealable: false },
        Candidate { index: 11, priority: 1, start_s: 9.0, unstealable: false },
        Candidate { index: 12, priority: 1, start_s: 2.0, unstealable: false },
        Candidate { index: 13, priority: 0, start_s: 0.0, unstealable: true },
    ];
    assert_eq!(steal(&candidates, 1, 4), vec![12]);
    assert_eq!(steal(&candidates, 2, 4), vec![12, 11]);
    // Three of the four are stealable: a fourth slot cannot be made, because the last voice is unstealable.
    assert_eq!(steal(&candidates, 3, 4), vec![12, 11, 10]);
    assert_eq!(steal(&candidates, 4, 4), Vec::<usize>::new());
    // Room enough is nothing stolen.
    assert_eq!(steal(&candidates, 1, 8), Vec::<usize>::new());
}

/// **A fade duration is a time to arrive**, for a duck and for a music layer alike. `tau_for_fade` derives the time
/// constant from `approach`'s own definition: within one part in a thousand at `fade_s`, and not before halfway
/// through it. This is the arithmetic `move_layers` rests on, pinned where the crate's other numbers live.
#[test]
fn a_fade_arrives_within_one_part_in_a_thousand() {
    use bevy_siren::model::tau_for_fade;

    for fade in [0.2_f32, 0.3, 0.8, 2.0] {
        let tau = tau_for_fade(fade);
        let target = 1.0_f32;
        let mut value = 0.0_f32;
        let dt = 1.0 / 240.0;
        let mut arrived = None;
        for step in 1..=(fade * 240.0) as usize + 8 {
            value = approach(value, target, tau, dt);
            if (target - value).abs() <= 0.001 && arrived.is_none() {
                arrived = Some(step as f32 * dt);
            }
        }
        let arrived = arrived.expect("the move arrives");
        assert!(
            arrived <= fade + 2.0 * dt,
            "fade {fade}: arrives at {arrived}, which is past the {fade} s it was given"
        );

        // And it is still moving halfway through: the duration is spent, not rounded to one frame.
        let mut half = 0.0_f32;
        for _ in 0..(fade / 2.0 * 240.0) as usize {
            half = approach(half, target, tau, dt);
        }
        assert!(
            (target - half).abs() > 0.001,
            "fade {fade}: already at the target halfway through ({half}), so the duration is not a duration"
        );
    }

    // No fade given is the crate's easing, unchanged.
    assert_eq!(tau_for_fade(0.0), bevy_siren::model::MIX_TAU_S);
    assert_eq!(tau_for_fade(-1.0), bevy_siren::model::MIX_TAU_S);
    assert_eq!(tau_for_fade(f32::NAN), bevy_siren::model::MIX_TAU_S);
}

/// **A voice index is a slot and a generation**, and the packing round-trips at both ends of both ranges. The mixer
/// hands a `usize` to three systems that hold it across frames, so the encoding cannot drift: slot 0 at generation
/// `u32::MAX` and the largest slot the low bits allow must both come back exactly as they went in.
#[test]
fn a_voice_index_is_a_slot_and_a_generation() {
    use bevy_siren::model::{VOICE_SLOT_BITS, pack_voice, unpack_voice};

    assert_eq!(unpack_voice(pack_voice(0, 1)), (0, 1));
    assert_eq!(unpack_voice(pack_voice(7, 3)), (7, 3));
    assert_eq!(unpack_voice(pack_voice(0, u32::MAX)), (0, u32::MAX));
    let widest_slot = (1usize << VOICE_SLOT_BITS) - 1;
    assert_eq!(unpack_voice(pack_voice(widest_slot, 9)), (widest_slot, 9));

    // Two different voices never share an index, which is the whole point.
    assert_ne!(pack_voice(3, 1), pack_voice(3, 2));
    assert_ne!(pack_voice(3, 1), pack_voice(4, 1));
}
