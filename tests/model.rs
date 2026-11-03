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

/// **The panner's own law is the number a spatial cue divides back out** — and the number a non-spatial cue used to be
/// handed unawares. On-axis, 8 m from a listener whose ears are a metre apart, `rodio`'s formula gives about 0.0117 in
/// amplitude, i.e. **−38.7 dB**; that is why routing a "flat" cue through the panner was not a pan but a silence.
#[test]
fn the_panner_law_is_what_a_spatial_cue_divides_out() {
    use bevy_siren::model::backend_spatial_gain;

    let gain = backend_spatial_gain([0.0, 0.0, 8.0], [-0.5, 0.0, 0.0], [0.5, 0.0, 0.0]);
    assert!((gain - 0.0117).abs() < 0.0005, "the panner's own gain at 8 m: {gain}");
    let db = 20.0 * gain.log10();
    assert!(
        (db - -38.7).abs() < 0.2,
        "which is {db} dB — a cue reported at 0 dB would be silent in practice"
    );
}

/// **A `0` seed reports the seed it used.** `Rng::new(0)` maps a zero state to `ZERO_SEED`, because xorshift's state must
/// never be zero; `SirenChoices` recorded the **argument**, so `seed()` — documented as *"the seed this stream was built
/// from"* — named a number the stream did not come from. `ZERO_SEED` is also the crate's default seed, so `seed: 0` and
/// the default are **one stream wearing two names**, and the report has to say which one.
#[test]
fn a_zero_seed_reports_the_seed_it_used() {
    use bevy_siren::SirenChoices;
    assert_eq!(
        SirenChoices::new(0).seed(),
        bevy_siren::model::Rng::ZERO_SEED,
        "the stream started at ZERO_SEED, so that is what it reports"
    );
    assert_eq!(
        bevy_siren::model::Rng::new(0).seed(),
        bevy_siren::model::Rng::ZERO_SEED,
        "and `Rng` says the same thing, because it owns the mapping"
    );
}

/// **A nonzero seed reports itself.** The guard: the fix is a report of a mapping, not a blanket rewrite of every seed.
#[test]
fn a_nonzero_seed_reports_itself() {
    use bevy_siren::SirenChoices;
    assert_eq!(SirenChoices::new(7).seed(), 7);
    assert_eq!(bevy_siren::model::Rng::new(7).seed(), 7);
}

/// **The seed does not change as the stream advances.** `seed()` reads the value the state started at, not the state
/// itself — the bug this test exists to prevent was reading the live state, which changes on every draw.
#[test]
fn a_streams_seed_does_not_move_as_it_draws() {
    let mut rng = bevy_siren::model::Rng::new(5);
    let before = rng.seed();
    for _ in 0..64 {
        let _ = rng.next_u32();
    }
    assert_eq!(rng.seed(), before, "64 draws later, the seed is still 5");
}

/// **A `0` seed and the crate's default seed are one stream.** Pinning the behaviour the report describes: a future change
/// that split them would be a behaviour change with its own decision, and this test makes that visible rather than quiet.
#[test]
fn a_zero_seed_and_the_default_seed_are_one_stream() {
    use bevy_siren::mix::SirenSettings;
    use bevy_siren::model::Rng;

    assert_eq!(Rng::new(0).seed(), SirenSettings::default_seed(), "the same number");
    let mut zero = Rng::new(0);
    let mut default = Rng::new(SirenSettings::default_seed());
    for i in 0..32 {
        assert_eq!(zero.next_u32(), default.next_u32(), "and the same stream, draw {i}");
    }
    let mut one = Rng::new(1);
    let mut zero = Rng::new(0);
    assert_ne!(zero.next_u32(), one.next_u32(), "while seed 1 is a different stream");
}

/// **A refusal outlives a burst of plays.** `SirenLog` evicted the oldest entry whatever it was, so one refusal was
/// pushed out by exactly `CAPACITY` plays — and a 32-voice scene at 20 footsteps a second writes 640 entries a second,
/// so the line explaining why nothing played survived about **0.4 s**. R6: *"a play is never silent: `Played` or
/// `Refused`, both with the reason"* — a refusal written and then thrown away is the silent failure with an extra step.
#[test]
fn a_refusal_outlives_a_burst_of_plays() {
    use bevy_siren::{Entry, SirenLog};

    let mut log = SirenLog::new();
    log.push(Entry::Refused { at_s: 0.0, cue: "typo".to_owned(), reason: "no cue of that name".to_owned() });
    for i in 0..SirenLog::CAPACITY {
        log.push(Entry::Played {
            at_s: i as f64,
            cue: format!("c{i}"),
            sound: "s".to_owned(),
            bus: "Effects".to_owned(),
            gain_db: 0.0,
        });
    }

    assert_eq!(log.len(), SirenLog::CAPACITY, "the buffer is still the panel's budget");
    let kept: Vec<&Entry> = log.iter().filter(|e| matches!(e, Entry::Refused { .. })).collect();
    assert_eq!(
        kept.len(),
        1,
        "and the refusal is still there: {:?}",
        log.iter().map(|e| e.line()).collect::<Vec<_>>()
    );
}

/// **The buffer never exceeds its capacity.** The guard: a new eviction rule must not grow the ring.
#[test]
fn the_buffer_never_exceeds_its_capacity() {
    use bevy_siren::{Entry, SirenLog};

    let mut log = SirenLog::new();
    for i in 0..1000u32 {
        if i % 7 == 0 {
            log.push(Entry::Refused { at_s: i as f64, cue: "c".to_owned(), reason: "r".to_owned() });
        } else {
            log.push(Entry::Stopped { at_s: i as f64, cue: "c".to_owned(), why: "ended".to_owned() });
        }
        assert!(log.len() <= SirenLog::CAPACITY, "at push {i}");
    }
    assert_eq!(log.len(), SirenLog::CAPACITY);
}

/// **A log of only refusals still evicts.** The degenerate case: if all 256 entries are refusals there is nothing routine
/// to drop, and keeping them all is impossible — so the oldest goes. 256 refusals is a game already failing loudly.
#[test]
fn a_log_of_only_refusals_still_evicts() {
    use bevy_siren::{Entry, SirenLog};

    let mut log = SirenLog::new();
    for i in 0..300u32 {
        log.push(Entry::Refused { at_s: i as f64, cue: format!("c{i}"), reason: "r".to_owned() });
    }
    assert_eq!(log.len(), SirenLog::CAPACITY, "300 refusals leave 256");
    let oldest = log.iter().next().expect("one entry");
    assert!(
        matches!(oldest, Entry::Refused { cue, .. } if cue == "c44"),
        "and the oldest kept is the 45th: {}",
        oldest.line()
    );
}

/// **Entries stay oldest-first.** A panel draws the column in `iter()` order, and the eviction rule removes from the
/// middle of the routine class, so the order of what remains is what this pins.
#[test]
fn entries_stay_in_order_after_an_eviction() {
    use bevy_siren::{Entry, SirenLog};

    let mut log = SirenLog::new();
    log.push(Entry::Refused { at_s: 0.0, cue: "first".to_owned(), reason: "r".to_owned() });
    for i in 1..=SirenLog::CAPACITY {
        log.push(Entry::Played {
            at_s: i as f64,
            cue: format!("c{i}"),
            sound: "s".to_owned(),
            bus: "Effects".to_owned(),
            gain_db: 0.0,
        });
    }
    let times: Vec<f64> = log
        .iter()
        .filter_map(|e| match e {
            Entry::Played { at_s, .. } => Some(*at_s),
            _ => None,
        })
        .collect();
    assert!(times.windows(2).all(|w| w[0] < w[1]), "oldest first: {times:?}");
    assert_eq!(times.first().copied(), Some(2.0), "and the first play (`c1`, at 1.0) is the one dropped");
}

/// **A played gain is written so a distance law can be read off it.** `cues.yaml` case 14 asserts that
/// `Attenuation::Inverse { near_m: 4, rolloff: 1 }` gives −6.02 dB at twice its near radius, and iteration 41 put that
/// number on the panel's cue row. `Entry::line` wrote one decimal, so the log said `-6.0` beside the row's `-6.02` —
/// two numbers for one fact, and at a precision where a law and a cue's own authored gain are the same string. The
/// mutation is `{gain_db:.1}`, which this test reddens.
#[test]
fn a_played_gain_is_written_to_two_decimals() {
    let entry = bevy_siren::Entry::Played {
        at_s: 0.5,
        cue: "bang".to_owned(),
        sound: "hit_01".to_owned(),
        bus: "Effects".to_owned(),
        gain_db: bevy_siren::model::gain_to_db(bevy_siren::model::distance_gain(
            bevy_siren::Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
            8.0,
        )),
    };
    let line = entry.line();
    assert!(line.contains("at -6.02 dB"), "one decimal cannot show the law: {line}");
    // The field itself keeps full precision; only the sentence rounds.
    match entry {
        bevy_siren::Entry::Played { gain_db, .. } => {
            assert!((gain_db - -6.020_6).abs() < 1e-3, "{gain_db}")
        }
        _ => unreachable!(),
    }
}

/// **The window a picker refuses is the number it was asked for, not the one its first play sized.**
///
/// `pick_from` takes `avoid` per call and `pick` reads it fresh from the cue, but the picker was built once, on the
/// cue's first play, and `remember` bounded itself by the deque's **allocator** capacity. So `capacity 1` asked for
/// `avoid: 3` produced the byte-identical sequence to `avoid: 1` — 24 picks, measured — and a table reload that raised
/// `avoid` changed nothing for the life of the app.
#[test]
fn a_raised_window_is_honoured() {
    let keys = ["a", "b", "c", "d"];
    let run = |picker: &mut VariantPicker, avoid: u8| -> String {
        let mut rng = Rng::new(11);
        (0..24).map(|_| keys[picker.pick_from(&keys, avoid, &mut rng).expect("a key")]).collect()
    };

    let mut once = VariantPicker::new(1);
    let asked_three = run(&mut once, 3);

    let mut built_for_three = VariantPicker::new(3);
    let expected = run(&mut built_for_three, 3);
    assert_eq!(
        asked_three, expected,
        "a picker asked for `avoid: 3` must refuse three variants; got {asked_three} against {expected}"
    );

    let mut one = VariantPicker::new(1);
    assert_ne!(asked_three, run(&mut one, 1), "and that window must not be `avoid: 1`'s");
}

/// **Growing reserves room and keeps every pick; shrinking keeps the newest.**
///
/// The rule refuses what was heard *recently*, so a narrower window must drop the **oldest** memory and not the newest.
#[test]
fn set_capacity_keeps_the_newest_picks() {
    let keys = ["a", "b", "c", "d"];
    let mut picker = VariantPicker::new(1);
    let mut rng = Rng::new(3);
    for _ in 0..8 {
        picker.pick_from(&keys, 1, &mut rng).expect("a key");
    }
    assert_eq!(picker.capacity(), 1);
    picker.set_capacity(4);
    assert_eq!(picker.capacity(), 4);
    picker.set_capacity(0);
    assert_eq!(picker.capacity(), 0);
    // A zero window refuses nothing, so a pick still answers.
    assert!(picker.pick_from(&keys, 0, &mut rng).is_some());
}

/// **A picker built at `Default` is the crate's default rule, not a picker that remembers nothing.**
///
/// `derive(Default)` left every field empty and `remember` returned early on a zero capacity, so a default picker
/// silently refused nothing at all.
#[test]
fn a_default_picker_remembers_one_pick() {
    assert_eq!(VariantPicker::default().capacity(), 1);
    let keys = ["a", "b"];
    let mut picker = VariantPicker::default();
    let mut rng = Rng::new(5);
    let seq: Vec<&str> =
        (0..12).map(|_| keys[picker.pick_from(&keys, 1, &mut rng).expect("a key")]).collect();
    for pair in seq.windows(2) {
        assert_ne!(pair[0], pair[1], "a default picker must never repeat: {seq:?}");
    }
}

/// **`avoid: 0` really does refuse nothing** — the behavioural half of `CueProblem::InertAvoidRepeat`.
///
/// The problem report says so; this proves it, because a report that disagreed with the picker would be worse than no
/// report. Two pickers with the same seed and the same keys must produce the same sequence when one is asked for zero.
#[test]
fn a_zero_avoid_window_refuses_exactly_nothing() {
    use bevy_siren::model::{Rng, VariantPicker};

    let keys = ["a", "b", "c"];
    let mut zero = VariantPicker::new(1);
    let mut zero_rng = Rng::new(7);
    let mut shuffle = VariantPicker::new(1);
    let mut shuffle_rng = Rng::new(7);

    let mut zero_picks = Vec::new();
    let mut shuffle_picks = Vec::new();
    for _ in 0..24 {
        zero_picks.push(zero.pick_from(&keys, 0, &mut zero_rng).expect("a pick"));
        shuffle_picks.push(shuffle.pick_from(&keys, 0, &mut shuffle_rng).expect("a pick"));
    }
    assert_eq!(zero_picks, shuffle_picks, "a window of zero is the same rule as `Shuffle`");

    // **And a zero window really can repeat back to back**, which is what the footstep rule exists to prevent. Over 24
    // draws on three keys a repeat somewhere is overwhelmingly likely; asserting it makes the claim concrete.
    assert!(
        zero_picks.windows(2).any(|w| w[0] == w[1]),
        "a zero window permits a repeat, which is the machine-gun footstep: {zero_picks:?}"
    );

    // **A window of one does not**, which is what the default asks for.
    let mut one = VariantPicker::new(1);
    let mut one_rng = Rng::new(7);
    let picks: Vec<usize> = (0..24).map(|_| one.pick_from(&keys, 1, &mut one_rng).expect("a pick")).collect();
    assert!(!picks.windows(2).any(|w| w[0] == w[1]), "`avoid: 1` never repeats back to back: {picks:?}");
}
