//! **A voice that belongs to an entity**: it ends when its owner does, and a placed one follows it.
//!
//! `Play::on(owner)` used to make the voice a *child* of its owner (`ChildOf`), and a child is a hierarchy node: it inherits its parent's transform, it is made through a hook that queues a command on the parent, and it is despawned with it. Run as a
//! game would run it, that was **a panic** for an owner that is allocated and not yet spawned (every `Commands::spawn` returns one), a sound **at the owner's transform composed with `at`** (12 dB quiet, or 52 dB over the sample for an owner with no
//! `Transform`, which Bevy never propagates), **a loop that lived for ever** on an owner that was already gone, and **a false *owner despawned* line** when the game despawned the voice itself
//! (`docs/loop/runs/2026-11-04-119/evidence/before/`).
//!
//! A voice now carries [`VoiceOwner`] — data, not a relationship: no hook runs, no transform is inherited, nothing is despawned on its account — and [`end_and_follow`] reads what the owner *is*, using the three states `Entities` tells apart
//! (Ousterhout, *A Philosophy of Software Design*, §10.3: define the error out of existence — an owner not spawned yet has not begun, one that is gone is already over):
//!
//! - **alive** — keep the voice; a *placed* voice follows its owner (below);
//! - **pending** — allocated and not spawned: the command that spawns it is queued and applies before the next frame, so wait;
//! - **gone** — the generation has passed: write `stopped: owner despawned` and despawn the voice, in the frame it was played if its owner was already gone.
//!
//! **Following.** `Play::at` is a *world* position, so the crate's own distance law, its compensation for the backend's panner, and the panner itself all see the same place. A placed voice on Bevy's path is kept at the same *offset in its owner's space*:
//! the first frame the owner's `GlobalTransform` has been **propagated** the offset is fixed (`inverse(owner) · the voice's world position`) — a `GlobalTransform` added since the system last ran is still the default, and an offset measured against it would
//! be wrong for ever, which an owner spawned in the same frame as the play meets — and every frame after, the voice's `Transform` is `owner ∘ offset`: what the hierarchy gave by accident, without the double transform. The owner's `GlobalTransform` is the
//! previous frame's (the crate writes in `Update`, Bevy propagates in `PostUpdate`), a lag of at most one frame. An owner with no `Transform` has no place to follow: its voice stays where it was put.

use bevy::ecs::entity::{Entities, EntityNotSpawnedError};
use bevy::prelude::*;

use crate::log::{Entry, SirenLog};
use crate::play::{Voice, VoiceOwner};

/// What an owner entity is right now: the three states [`Entities`] tells apart, and `ChildOf` could not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnerState {
    /// Spawned.
    Alive,
    /// Allocated and not yet spawned: the command that spawns it is queued. It is what `Commands::spawn` returns before the command applies.
    Pending,
    /// Despawned: its generation has passed.
    Gone,
}

/// Which of the three `owner` is — `Entities::get_spawned` (`bevy_ecs-0.20.1/src/entity/mod.rs:846`) says *invalid* for a passed generation and *valid but not spawned* for an allocated id.
pub(crate) fn state_of(entities: &Entities, owner: Entity) -> OwnerState {
    match entities.get_spawned(owner) {
        Ok(_) => OwnerState::Alive,
        Err(EntityNotSpawnedError::ValidButNotSpawned(_)) => OwnerState::Pending,
        Err(EntityNotSpawnedError::Invalid(_)) => OwnerState::Gone,
    }
}

/// Where a placed voice sits relative to its owner, in the owner's space — `None` until the first frame the owner's `GlobalTransform` has been propagated, so there is something real to measure against. Inserted for a voice that has an owner, is placed (a spatial cue **and** a position)
/// and is played on Bevy's path: the crate's own mixer does not pan and applies the distance law once, at the play, so there is nothing for it to follow.
#[derive(Component, Default, Debug)]
pub(crate) struct Follows(Option<Vec3>);

/// What the system reads and writes of a voice that has an owner: its identity and cue (for the log), the owner, the follow offset, and the transform it keeps at the owner's side.
type OwnedVoice<'a> = (Entity, &'a Voice, &'a VoiceOwner, Option<&'a mut Follows>, Option<&'a mut Transform>);

/// **Ends every voice whose owner is gone, and keeps every placed voice at its owner's side.** Runs in [`SirenSystems::Play`](crate::SirenSystems) directly after `play_requests`, so a voice on an owner that is already gone is despawned
/// *before* `start_pending` can start it on the crate's own mixer (a start and a stop for a sound that was never to sound is a blip).
///
/// A gone owner's voice is despawned at once, not faded — the behaviour the hierarchy gave (a cascade) and `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` pins. A voice the *game* despawns is none of this system's business: nothing
/// here blames an owner for it.
pub(crate) fn end_and_follow(
    mut commands: Commands,
    entities: &Entities,
    real: Res<Time<Real>>,
    mut log: ResMut<SirenLog>,
    mut voices: Query<OwnedVoice>,
    owners: Query<Ref<GlobalTransform>>,
) {
    let now = real.elapsed_secs_f64();
    for (entity, voice, owner, follows, transform) in &mut voices {
        match state_of(entities, owner.0) {
            OwnerState::Alive => {}
            OwnerState::Pending => continue,
            OwnerState::Gone => {
                log.push(Entry::Stopped {
                    at_s: now,
                    cue: voice.cue.clone(),
                    why: "owner despawned".to_owned(),
                });
                commands.entity(entity).despawn();
                continue;
            }
        }
        let (Some(mut follows), Some(mut transform), Ok(owner_at)) =
            (follows, transform, owners.get(owner.0))
        else {
            continue;
        };
        match follows.0 {
            None => {
                // **A `GlobalTransform` added since this system last ran has not been propagated yet** — Bevy propagates in `PostUpdate` — so it is the default, and an offset measured against it would be wrong for ever (the sound at owner ∘ `at` from
                // the next frame: the double transform again). An owner spawned in the same frame as the play meets this. Wait a frame.
                if owner_at.is_added() {
                    continue;
                }
                // The voice's `Transform` is its world position — it is not a child — so the offset is the owner's inverse applied to it. A degenerate owner (zero scale) has no inverse: try again next frame.
                let offset = owner_at.affine().inverse().transform_point3(transform.translation);
                if offset.is_finite() {
                    follows.0 = Some(offset);
                }
            }
            Some(offset) => {
                let at = owner_at.transform_point(offset);
                if at.is_finite() && transform.translation != at {
                    transform.translation = at;
                }
            }
        }
    }
}
