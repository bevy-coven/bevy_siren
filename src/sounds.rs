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

    /// **Drops every key the table no longer names.**
    ///
    /// [`load_table`](Self::load_table) merges — it skips a key it already holds and never removes one — and the
    /// resource had no remover at all, so a project that reloaded a table with renamed keys kept one handle per key it
    /// had **ever** loaded. A handle is what keeps an `AudioSource` alive, and an `AudioSource` holds the whole encoded
    /// file (`bytes: Arc<[u8]>`), so the stale keys were whole files resident for the life of the process.
    ///
    /// Measured: two tables of ten different keys loaded in turn left **twenty** keys held.
    ///
    /// **Call this before [`load_table`](Self::load_table) on a reload**, not after: the keys the new table keeps are
    /// dropped then re-asked (one `AssetServer::load` per kept key, which the asset server already has cached), while
    /// the keys it lost are freed. After would work too, but would keep a lost file alive across the load.
    pub fn retain_keys(&mut self, table: &CueTable) {
        if self.handles.is_empty() {
            return;
        }
        let live: std::collections::HashSet<&str> = table.sounds().into_iter().collect();
        self.handles.retain(|key, _| live.contains(key.as_str()));
    }

    /// Every `sound` a cue names that has no handle yet: the editor shows this, and a play refuses it.
    pub fn missing(&self, table: &CueTable) -> Vec<String> {
        table.sounds().into_iter().filter(|key| !self.handles.contains_key(*key)).map(str::to_owned).collect()
    }

    /// Every key whose load has failed, so a project can say which file to deliver.
    ///
    /// **The crate asks the same question where it decides**, so a game need not remember to poll this: a play whose picked sound
    /// is in this list **and whose bytes are not loaded** is refused (`Refusal::MissingSound`), and a voice whose load fails after it was
    /// played is refused and freed — both with the asset server's own sentence in the log. (A key whose *reload* failed is listed here
    /// and still plays, because the previous bytes are loaded.)
    pub fn failed(&self, server: &AssetServer, table: &CueTable) -> Vec<String> {
        table
            .sounds()
            .into_iter()
            .filter(|key| self.handles.get(*key).is_some_and(|handle| load_error(server, handle).is_some()))
            .map(str::to_owned)
            .collect()
    }
}

/// **Why `handle`'s file failed to load**, in the asset server's own words (the cause and the path it looked for), or `None` when it has not failed.
///
/// `None` for a load still in flight, for a loaded asset, and for a handle the server has no record of — a reserved handle, a default (Uuid) one:
/// `AssetServer::get_load_state` answers `None` for a Uuid handle (*"Always say we don't have Uuid assets"*, `bevy_asset-0.20.1/src/server/mod.rs:1075`),
/// and nothing is invented about a file the server never heard of. This is the one place the crate reads `LoadState::Failed`.
pub(crate) fn load_error(server: &AssetServer, handle: &Handle<AudioSource>) -> Option<String> {
    match server.get_load_state(handle.id()) {
        Some(bevy::asset::LoadState::Failed(error)) => Some(error.to_string()),
        _ => None,
    }
}

/// **What the asset server and the asset table say about a sound's file**, as one system parameter — `play_requests` is at Bevy's sixteen, and a tuple of the two resources is one parameter that adds no type to its public
/// signature. Both are optional: a world with no `AssetPlugin` has no server and no table, and then no file can have failed.
pub(crate) type Files<'w> = (Option<Res<'w, AssetServer>>, Option<Res<'w, Assets<AudioSource>>>);

/// **Why the sound behind `handle` cannot be played because its file failed to load**, or `None` — which also means *its bytes are not there*.
///
/// That second condition is the difference from [`load_error`]. A **reload** that fails — an edited file half-way through saving — leaves the server's state `Failed` and the *previous* bytes in `Assets`
/// (a failed reload removes nothing), and a sound whose bytes are there can be played: refusing it would silence a cue the game could still sound. So a sound is refused for its file only when there is nothing to play.
pub(crate) fn cannot_load(files: &Files, handle: &Handle<AudioSource>) -> Option<String> {
    let (server, assets) = files;
    if assets.as_ref().is_some_and(|assets| assets.get(handle).is_some()) {
        return None;
    }
    load_error(server.as_deref()?, handle)
}

/// The log's sentence for a file that failed to load: the key a project maps to a file, and the asset server's own cause, which carries the path it looked for — the file to fix.
pub(crate) fn failed_to_load(sound: &str, error: &str) -> String {
    format!("the file for sound `{sound}` failed to load: {error}")
}
