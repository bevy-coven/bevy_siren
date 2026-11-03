//! `#37`: what a busy GPU does to an indexing run — refuse politely, wait a bounded time, and keep queries working.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::budget::{Budget, BudgetProblem, Deferral, ModelCost, WaitState, defer};

/// A device with `free` bytes uncommitted out of 24 GiB — the RTX 3090's own figure.
fn device_with_free(free: u64) -> Budget {
    Budget {
        total_bytes: 24 * 1024 * 1024 * 1024,
        used_bytes: 24 * 1024 * 1024 * 1024 - free,
        ceiling_bytes: None,
    }
}

/// CLAP's audio tower as this crate measured it.
fn cost() -> ModelCost {
    ModelCost::measured_clap_audio()
}

/// **Criterion 1: a load with insufficient free VRAM is refused naming the request and the availability.**
#[test]
fn a_short_device_is_refused_naming_both_numbers() {
    let short = device_with_free(60 * 1024 * 1024); // 60 MB free against a request of ~628 MB
    let problem = short.fits(cost()).expect_err("it cannot fit");

    match problem {
        BudgetProblem::DoesNotFit { need_bytes, available_bytes } => {
            assert_eq!(need_bytes, cost().total_bytes(), "the request is named");
            assert_eq!(available_bytes, 60 * 1024 * 1024, "and so is the availability");
        }
        other => panic!("expected DoesNotFit, got {other:?}"),
    }
    // The sentence a user reads carries both in MB.
    let said = problem.to_string();
    // **The number in the sentence is the one the struct carries**, formatted the way `Display` formats it — asserted
    // against the computed value rather than a guess at it. `ModelCost::measured_clap_audio` is 116.5 MB of weights plus
    // the 512 MiB overhead, which is 623 MB, not the 628 a hurried reading of the two constants suggests.
    let need_mb = (cost().total_bytes() as f64 / 1_048_576.0).round() as u64;
    assert_eq!(need_mb, 623, "116.5 MB of weights plus the 512 MiB overhead");
    assert!(said.contains(&format!("{need_mb} MB")), "the request is in the sentence: {said}");
    assert!(said.contains("60 MB"), "and so is the availability: {said}");
    assert!(said.contains("Close what is using the GPU"), "and what to do: {said}");
}

/// **Criterion 2: the policy is stated and observable** — a device that frees after two polls succeeds on the third
/// attempt, and the state said it was waiting.
#[test]
fn a_device_that_frees_is_caught_on_the_next_poll() {
    let mut frees_after = 2;
    let mut states = Vec::new();
    let mut sleeps = 0;

    let attempt = defer(
        cost(),
        Deferral::default(),
        || {
            if frees_after == 0 {
                device_with_free(8 * 1024 * 1024 * 1024) // 8 GB free: room, and then some
            } else {
                frees_after -= 1;
                device_with_free(60 * 1024 * 1024)
            }
        },
        |_seconds| sleeps += 1,
        |state| states.push(state),
    )
    .expect("it fits on the third look");

    assert_eq!(attempt, 3, "the attempt that succeeded, counting from 1");
    assert_eq!(sleeps, 2, "one poll between each of three looks");
    assert_eq!(
        states,
        vec![
            WaitState::Waiting { attempt: 2, of: 60 },
            WaitState::Waiting { attempt: 3, of: 60 },
            WaitState::Ready
        ]
    );
    // The state a panel renders says what it is doing, and stops saying it once ready.
    assert_eq!(
        WaitState::Waiting { attempt: 2, of: 60 }.describe().as_deref(),
        Some("waiting for the GPU (2 of 60)")
    );
    assert!(WaitState::Waiting { attempt: 2, of: 60 }.is_waiting());
    assert_eq!(WaitState::Ready.describe(), None);
}

/// **Criterion 2, the other half: it gives up after exactly the stated number of attempts**, not forever and not at once.
#[test]
fn a_device_that_stays_busy_is_refused_after_the_stated_bound() {
    let deferral = Deferral::default();
    let looks = std::cell::Cell::new(0u32);
    let mut states = Vec::new();
    let mut slept_for = 0.0f32;

    let problem = defer(
        cost(),
        deferral,
        || {
            looks.set(looks.get() + 1);
            device_with_free(60 * 1024 * 1024)
        },
        |seconds| slept_for += seconds,
        |state| states.push(state),
    )
    .expect_err("it never fits");

    assert_eq!(looks.get(), deferral.attempts(), "exactly the stated number of looks");
    assert_eq!(deferral.attempts(), 60, "30 s at one poll per 0.5 s");
    // The wait is bounded by the field, not by wall-clock luck: 59 polls of 0.5 s is 29.5 s, inside the 30 s bound.
    assert!((slept_for - 29.5).abs() < 1e-3, "slept {slept_for} s");
    assert!(slept_for <= deferral.wait_s, "the bound is respected");
    // **An expired deferral is a named refusal**, never an empty success.
    assert!(matches!(problem, BudgetProblem::DoesNotFit { .. }), "got {problem:?}");
    assert_eq!(states.last(), Some(&WaitState::GaveUp));
    assert!(WaitState::GaveUp.describe().expect("a sentence").contains("stayed busy"));
}

/// **Criterion 3: a query that needs no GPU still works while indexing is deferred.**
///
/// The deferral is driven to completion inside this test, and a real index query runs between two of its polls — which
/// is the property that matters: deferral must not block the process or take the index away.
#[test]
fn a_query_still_answers_while_the_build_waits() {
    use bevy_siren::semantic::index::{Index, ModelStamp};
    use bevy_siren::semantic::rank::{Floor, top_k};

    let stamp = ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 };
    let mut index = Index::new(stamp);
    let _ = index.insert(1, vec![1.0, 0.0, 0.0]);
    let _ = index.insert(2, vec![0.0, 1.0, 0.0]);

    let mut frees_after = 3;
    let mut answered_while_waiting = 0usize;

    let _ = defer(
        cost(),
        Deferral::default(),
        || {
            frees_after -= 1;
            if frees_after <= 0 {
                device_with_free(8 * 1024 * 1024 * 1024)
            } else {
                device_with_free(60 * 1024 * 1024)
            }
        },
        |_seconds| {
            // **A query runs between two polls.** This is the criterion: no GPU is needed to answer it, so a busy card
            // must not take the index away.
            let ranked = top_k(&index, &[1.0, 0.0, 0.0], 2).expect("a ranking with no GPU");
            assert_eq!(ranked[0].hash, 1);
            answered_while_waiting += 1;
        },
        |_state| {},
    )
    .expect("room appears");
    assert!(answered_while_waiting >= 2, "queries answered during the wait: {answered_while_waiting}");
    // And the index is intact afterwards.
    assert_eq!(top_k(&index, &[1.0, 0.0, 0.0], 2).expect("still queryable").len(), 2);
    let _ = Floor::default();
}

/// A device with room on the first look reports `Ready` and never sleeps.
#[test]
fn a_device_with_room_does_not_wait_at_all() {
    let mut states = Vec::new();
    let mut sleeps = 0;
    let attempt = defer(
        cost(),
        Deferral::default(),
        || device_with_free(8 * 1024 * 1024 * 1024),
        |_| sleeps += 1,
        |state| states.push(state),
    )
    .expect("it fits at once");
    assert_eq!(attempt, 1);
    assert_eq!(sleeps, 0, "no poll is taken when there is nothing to wait for");
    assert!(states.is_empty(), "and nothing is reported, because nothing is happening");
}

/// The attempt count is derived from the two fields, so they cannot disagree — including the degenerate settings.
#[test]
fn the_attempt_count_is_derived_and_never_zero() {
    assert_eq!(Deferral::default().attempts(), 60);
    assert_eq!(Deferral { wait_s: 1.0, poll_s: 0.5 }.attempts(), 2);
    // **A poll of zero would divide by it**, and a look is still owed: one attempt rather than an infinity.
    assert_eq!(Deferral { wait_s: 30.0, poll_s: 0.0 }.attempts(), 1);
    assert_eq!(Deferral { wait_s: -5.0, poll_s: 1.0 }.attempts(), 1);
    assert_eq!(Deferral { wait_s: f32::NAN, poll_s: 1.0 }.attempts(), 1);
}

/// **`Session::admit_deferring` is the policy wired to a real session**: a busy device is waited out, and the session
/// that comes back carries the cost the wait was for.
#[test]
fn a_session_is_admitted_after_waiting_for_the_gpu() {
    use bevy_siren::semantic::session::{Loaded, Session};

    let loaded = Loaded {
        inputs: vec![String::from("input")],
        outputs: vec![String::from("embedding")],
        dimension: 512,
        dynamic_batch: false,
    };
    let mut frees_after = 2;
    let mut saw_waiting = false;

    let session = Session::admit_deferring(
        cost(),
        Deferral::default(),
        || {
            if frees_after == 0 {
                device_with_free(8 * 1024 * 1024 * 1024)
            } else {
                frees_after -= 1;
                device_with_free(60 * 1024 * 1024)
            }
        },
        |_| {},
        |state| {
            if state.is_waiting() {
                saw_waiting = true;
            }
        },
        loaded,
        512,
    )
    .expect("admitted once room appeared");

    assert!(saw_waiting, "the wait was reported, which is what a panel renders");
    assert_eq!(session.cost(), cost(), "the session knows what it committed");
    assert_eq!(session.loaded().dimension, 512);
}

/// **A wrong graph is reported as a wrong graph, even when the device was busy first** — the wait does not swallow the
/// dimension mismatch into a budget problem it has nothing to do with.
#[test]
fn a_wrong_dimension_is_still_named_after_the_wait() {
    use bevy_siren::semantic::session::{LoadProblem, Loaded, Session};

    let loaded = Loaded {
        inputs: vec![String::from("input")],
        outputs: vec![String::from("embedding")],
        dimension: 768,
        dynamic_batch: false,
    };
    let problem = Session::admit_deferring(
        cost(),
        Deferral::default(),
        || device_with_free(8 * 1024 * 1024 * 1024),
        |_| {},
        |_| {},
        loaded,
        512,
    )
    .expect_err("768 is not 512");
    assert!(
        matches!(problem, LoadProblem::DimensionMismatch { graph: 768, expected: 512 }),
        "got {problem:?}"
    );
}
