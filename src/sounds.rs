//! **What a cue's `sound` key means**: an opaque name the game maps to a loaded sample.
//!
//! A key is never a path the crate guesses. `"footstep.grass"` becomes whatever handle the game inserts;
//! [`SirenSounds::load_table`] is the convenience for the common case where the key and the file share
//! one name under one directory. That split is what lets a cue table be written before an artist
//! delivers anything, and lets one recording replace another without touching the table — Böttcher's
//! point about procedural audio (2008) applied the other way round: the *asset workflow* is the hard
//! part, so the crate keeps the mapping in one place a project can fill by hand or by loader.

use std::collections::BTreeMap;

use bevy::asset::AssetServer;
use bevy::audio::AudioSource;
use bevy::prelude::*;

use crate::cue::CueTable;

/// The keys a project's cues name, and the handles they resolve to.
#[derive(Resource, Default, Clone, Debug)]
pub struct SirenSounds {
    handles: BTreeMap<String, Handle<AudioSource>>,
}

impl SirenSounds {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Maps `key` to `handle`, returning the handle it replaced.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        handle: Handle<AudioSource>,
    ) -> Option<Handle<AudioSource>> {
        self.handles.insert(key.into(), handle)
    }

    /// The handle for `key`.
    pub fn get(&self, key: &str) -> Option<&Handle<AudioSource>> {
        self.handles.get(key)
    }

    /// How many keys resolve.
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// True when no key resolves.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Every key mapped, in key order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.handles.keys().map(|k| k.as_str())
    }

    /// Asks the asset server for `<dir>/<key>` for every distinct `sound` in `table`, and returns the
    /// keys it asked for that it had no handle for yet.
    ///
    /// A load is asynchronous, so a missing file cannot be reported here: the server logs the 404 and
    /// [`failed`](Self::failed) names it once the load has resolved. `dir` is empty for the asset root,
    /// and a trailing slash is not needed.
    pub fn load_table(&mut self, server: &AssetServer, table: &CueTable, dir: &str) -> Vec<String> {
        let mut asked = Vec::new();
        for key in table.sounds() {
            if self.handles.contains_key(key) {
                continue;
            }
            let path =
                if dir.is_empty() { key.to_owned() } else { format!("{}/{key}", dir.trim_end_matches('/')) };
            self.handles.insert(key.to_owned(), server.load::<AudioSource>(path));
            asked.push(key.to_owned());
        }
        asked
    }

    /// Every `sound` a cue names that has no handle yet: the editor shows this, and a play refuses it.
    pub fn missing(&self, table: &CueTable) -> Vec<String> {
        table.sounds().into_iter().filter(|key| !self.handles.contains_key(*key)).map(str::to_owned).collect()
    }

    /// Every key whose load has failed, so a project can say which file to deliver.
    pub fn failed(&self, server: &AssetServer, table: &CueTable) -> Vec<String> {
        table
            .sounds()
            .into_iter()
            .filter(|key| {
                self.handles.get(*key).is_some_and(|handle| {
                    matches!(server.get_load_state(handle.id()), Some(bevy::asset::LoadState::Failed(_)))
                })
            })
            .map(str::to_owned)
            .collect()
    }
}
