//! **What a cue's `sound` key means**: an opaque name the game maps to a loaded sample.
//!
//! A key is never a path the crate guesses. `"footstep.grass"` becomes whatever handle the game inserts;
//! [`SirenSounds::load_table`] is the convenience for the common case where the key and the file share
//! one name under one directory. That split is what lets a cue table be written before an artist
//! delivers anything, and lets one recording replace another without touching the table — Böttcher's
//! point about procedural audio (2008) applied the other way round: the *asset workflow* is the hard
//! part, so the crate keeps the mapping in one place a project can fill by hand or by loader.

use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::sync::{Arc, Mutex, PoisonError};

use bevy::asset::AssetServer;
use bevy::audio::{AudioSource, Decodable};
use bevy::prelude::*;

use crate::cue::CueTable;
use crate::output::Output;

/// The keys a project's cues name, and the handles they resolve to.
///
/// It also remembers, per asset, **what the decoder said about the bytes** (`Verdicts`): one decoder construction for a sound however often it is played and however many of the crate's systems ask, behind a `Mutex` so those
/// systems hold the map as a `Res` — one memo for Bevy's decoder and one for the crate's own mixer's, so the answer is always the asker's. A clone starts with an empty memory — it is a cache, not data.
#[derive(Resource, Default)]
pub struct SirenSounds {
    handles: BTreeMap<String, Handle<AudioSource>>,
    verdicts: Mutex<(Verdicts, Verdicts)>,
}

impl Clone for SirenSounds {
    fn clone(&self) -> Self {
        Self { handles: self.handles.clone(), verdicts: Mutex::default() }
    }
}

impl std::fmt::Debug for SirenSounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SirenSounds").field("handles", &self.handles).finish_non_exhaustive()
    }
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

/// What identifies a byte buffer **as a buffer**: where it is, how long it is, and how it begins. An edit — a hot reload — replaces an asset's `Arc<[u8]>`, so a new buffer has a new address even when it is the same length,
/// and the first 16 bytes (a format's magic) close the one hole an address alone leaves: a buffer freed and another of the same length put where it was, with a different header. A design default.
type BytesKey = (usize, usize, [u8; 16]);

/// **The decoder's answer for each asset one asker has been asked about**: the buffer it was asked about, and `None` for bytes the decoder reads or the decoder's own words for bytes it refuses. Held by [`SirenSounds`], one map
/// per asker (Bevy's decoder; the crate's own mixer's), and made of std and Bevy types only, so that nothing crate-private reaches `play_requests`'s public signature (iteration 116's `private_interfaces` lesson).
type Verdicts = HashMap<AssetId<AudioSource>, (BytesKey, Option<Arc<str>>)>;

/// **What the crate asks the asset server and the asset table about a sound's file**, as one system parameter — `play_requests` is at Bevy's sixteen, and a tuple is one parameter that adds no type to its public signature. Both
/// are optional: a world with no `AssetPlugin` has no server and no table, and then no file can have failed and there is nothing to decode.
pub(crate) type Files<'w> = (Option<Res<'w, AssetServer>>, Option<Res<'w, Assets<AudioSource>>>);

/// **Who would decode these bytes**, and so who is asked whether they can be decoded.
pub(crate) enum Decode<'a> {
    /// Nobody is asked: the crate's own mixer owns a voice that is still waiting for its start, which `start_pending` answers for.
    Skip,
    /// Bevy's `AudioSource::decoder`, which unwraps (`bevy_audio-0.20.1/src/audio_source.rs:96-101`): bytes it refuses are a panic in `PostUpdate`.
    Bevy,
    /// The crate's own mixer, asked through the output: bytes its `play` would refuse after `Played` was written.
    Output(&'a Output),
}

/// **Why a sound cannot be played**, and what the log says about it.
pub(crate) enum Unplayable {
    /// The asset server says the file failed to load, and nothing is loaded.
    FailedToLoad(String),
    /// The bytes are loaded and the decoder refuses them: its own words, and — when the first bytes say what the file is — a hint.
    Undecodable { error: Arc<str>, hint: Option<&'static str> },
}

impl Unplayable {
    /// The log's sentence: the key a project maps to a file, and the cause.
    pub(crate) fn sentence(&self, sound: &str) -> String {
        match self {
            Self::FailedToLoad(error) => failed_to_load(sound, error),
            Self::Undecodable { error, hint } => match hint {
                Some(hint) => format!("the bytes of sound `{sound}` cannot be decoded: {error} — {hint}"),
                None => format!("the bytes of sound `{sound}` cannot be decoded: {error}"),
            },
        }
    }
}

/// **Why the sound behind `handle` cannot be played**, or `None`.
///
/// *The file failed to load* and there is nothing to play: the asset server says so (iteration 116). A **reload** that fails — an edited file half-way through saving — leaves the server's state `Failed` and the *previous* bytes in
/// `Assets` (a failed reload removes nothing), and a sound whose bytes are there can be played, so it is not refused for its file.
///
/// *The bytes are there and the decoder refuses them* — asked of **whoever will decode them** (`decode`): Bevy's `AudioSource::decoder` is `….build().unwrap()` (`bevy_audio-0.20.1/src/audio_source.rs:96-101`), so a voice handed such bytes
/// panics the game in `PostUpdate` (iteration 117); the crate's own mixer refuses them in `start_pending`, **after `Played`** — a second answer to one play, and an entity despawned in the frame it was announced (iteration 118).
pub(crate) fn cannot_play(
    files: &Files,
    sounds: &SirenSounds,
    handle: &Handle<AudioSource>,
    decode: &Decode,
) -> Option<Unplayable> {
    let (server, assets) = files;
    if let Some(assets) = assets.as_deref()
        && let Some(source) = assets.get(handle)
    {
        if matches!(decode, Decode::Skip) {
            return None;
        }
        let error = sounds.verdict(assets, handle.id(), source, decode)?;
        return Some(Unplayable::Undecodable { error, hint: explain(&source.bytes) });
    }
    load_error(server.as_deref()?, handle).map(Unplayable::FailedToLoad)
}

/// The log's sentence for a file that failed to load: the key a project maps to a file, and the asset server's own cause, which carries the path it looked for — the file to fix.
pub(crate) fn failed_to_load(sound: &str, error: &str) -> String {
    format!("the file for sound `{sound}` failed to load: {error}")
}

/// **Exactly the call Bevy makes** to decode a sound (`bevy_audio-0.20.1/src/audio_source.rs:96-101`), minus the `unwrap` — made through `Decodable`'s associated type, so the crate names no `rodio` and the default build
/// gains no dependency. The decoder is whatever this build has: `rodio` with Bevy's features, plus the crate's own when `mixer` or `semantic` is on, so the answer can never disagree with the decoder that would run.
fn decoder_error(source: &AudioSource) -> Option<String> {
    #[cfg(test)]
    DECODED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(Arc::as_ptr(&source.bytes).cast::<u8>().addr());
    <AudioSource as Decodable>::Decoder::builder()
        .with_byte_len(source.bytes.len() as u64)
        .with_data(Cursor::new(source.clone()))
        .build()
        .err()
        .map(|error| error.to_string())
}

/// **Test only**: the address of every byte buffer the crate has built a decoder for, so a test can count the constructions for one sound (`a_sound_is_decoded_once_by_the_crate_however_often_it_is_played`).
#[cfg(test)]
static DECODED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// What a refusal can say about the file that the decoder does not: its words are *"The format of the data has not been recognized"* for all of these, and the three commonest deliveries can be told apart by their first
/// bytes. **A hint changes a sentence and never a decision** — the decoder decides.
///
/// - **no bytes**;
/// - **a Git LFS pointer**, the three-line text a repository stores where the audio should be (`version https://git-lfs.github.com/spec/v1`, `oid`, `size`);
/// - **an Ogg whose first packet is `OpusHead`** (RFC 7845 §5.1, the ID header of *Opus in Ogg*), which is not Vorbis and which this build's Vorbis decoder does not read.
fn explain(bytes: &[u8]) -> Option<&'static str> {
    if bytes.is_empty() {
        return Some("the file has no bytes");
    }
    if bytes.starts_with(b"version https://git-lfs.github.com/spec/") {
        return Some("a Git LFS pointer, not the audio file");
    }
    // The first packet of an Ogg stream starts after the 27-byte page header and its segment table (`bytes[26]` entries).
    if bytes.starts_with(b"OggS")
        && let Some(&segments) = bytes.get(26)
        && bytes.get(27 + usize::from(segments)..).is_some_and(|rest| rest.starts_with(b"OpusHead"))
    {
        return Some("an Ogg Opus file, and this build decodes Ogg Vorbis, not Opus");
    }
    None
}

/// Where a byte buffer is, how long, and how it begins — [`BytesKey`].
fn bytes_key(bytes: &Arc<[u8]>) -> BytesKey {
    let mut head = [0u8; 16];
    let taken = bytes.len().min(head.len());
    head[..taken].copy_from_slice(&bytes[..taken]);
    (Arc::as_ptr(bytes).cast::<u8>().addr(), bytes.len(), head)
}

impl SirenSounds {
    /// **The decoder's answer for these bytes, asked once per buffer and per asker.** A buffer the crate has already asked about is answered from the memo; an edited asset is a new buffer and is asked again. A memo is pruned (ids no
    /// longer in `Assets` dropped) when it outgrows twice the asset count plus 16, so a game that makes and drops audio does not grow it — both numbers design defaults.
    fn verdict(
        &self,
        assets: &Assets<AudioSource>,
        id: AssetId<AudioSource>,
        source: &AudioSource,
        decode: &Decode,
    ) -> Option<Arc<str>> {
        let mut memos = self.verdicts.lock().unwrap_or_else(PoisonError::into_inner);
        match decode {
            Decode::Skip => None,
            Decode::Bevy => verdict_with(&mut memos.0, assets, id, source, decoder_error),
            Decode::Output(output) => {
                verdict_with(&mut memos.1, assets, id, source, |source| output.0.decode_error(&source.bytes))
            }
        }
    }
}

/// [`verdict`], with the question a parameter so the memo can be tested without a decoder.
fn verdict_with(
    verdicts: &mut Verdicts,
    assets: &Assets<AudioSource>,
    id: AssetId<AudioSource>,
    source: &AudioSource,
    ask: impl FnOnce(&AudioSource) -> Option<String>,
) -> Option<Arc<str>> {
    let key = bytes_key(&source.bytes);
    if let Some((known, answer)) = verdicts.get(&id)
        && *known == key
    {
        return answer.clone();
    }
    if verdicts.len() > 2 * assets.len() + 16 {
        verdicts.retain(|id, _| assets.contains(*id));
    }
    let answer = ask(source).map(Arc::from);
    verdicts.insert(id, (key, answer.clone()));
    answer
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn source(bytes: &[u8]) -> AudioSource {
        AudioSource { bytes: Arc::from(bytes) }
    }

    /// **One buffer is asked once; an edited one is asked again** — even when the edit keeps the length and the head, because the buffer is a different allocation.
    #[test]
    fn a_buffer_is_asked_once_and_an_edit_is_asked_again() {
        let mut assets = Assets::<AudioSource>::default();
        let handle = assets.add(source(&[7u8; 24]));
        let id = handle.id();
        let asked = Cell::new(0);
        let ask = |_: &AudioSource| {
            asked.set(asked.get() + 1);
            Some(format!("asked {} times", asked.get()))
        };
        let mut verdicts = Verdicts::new();

        let first = assets.get(id).expect("the asset");
        let a = verdict_with(&mut verdicts, &assets, id, first, ask);
        let b = verdict_with(&mut verdicts, &assets, id, first, ask);
        assert_eq!(
            (asked.get(), a.as_deref(), b.as_deref()),
            (1, Some("asked 1 times"), Some("asked 1 times"))
        );

        // The same length and the same first 16 bytes, a different buffer: an edit.
        assets.insert(id, source(&[[7u8; 16].as_slice(), [9u8; 8].as_slice()].concat())).expect("replaced");
        let edited = assets.get(id).expect("the asset");
        let c = verdict_with(&mut verdicts, &assets, id, edited, ask);
        assert_eq!((asked.get(), c.as_deref()), (2, Some("asked 2 times")), "a new buffer is a new question");
    }

    /// **The memo does not grow with the assets a game has dropped**: past twice the live count plus 16, the verdicts of ids no longer in `Assets` go.
    #[test]
    fn the_memo_forgets_assets_that_are_gone() {
        let mut assets = Assets::<AudioSource>::default();
        let mut verdicts = Verdicts::new();
        for n in 0..100u8 {
            let handle = assets.add(source(&[n; 20]));
            let id = handle.id();
            let held = assets.get(id).expect("the asset");
            verdict_with(&mut verdicts, &assets, id, held, |_| None);
            // A game that makes audio and drops it: the asset goes at once.
            assets.remove(id);
        }
        assert!(
            verdicts.len() < 100,
            "ids that are gone were forgotten along the way: {} kept",
            verdicts.len()
        );

        let keep = assets.add(source(b"kept"));
        let held = assets.get(keep.id()).expect("the asset");
        verdict_with(&mut verdicts, &assets, keep.id(), held, |_| None);
        assert!(
            verdicts.len() <= 2 * assets.len() + 16 + 1,
            "bounded by the live assets: {} kept",
            verdicts.len()
        );
    }

    /// The hints, each from its first bytes — and **never** from anything but its first bytes.
    #[test]
    fn a_hint_comes_from_the_first_bytes_and_only_when_they_say_what_the_file_is() {
        assert_eq!(explain(b""), Some("the file has no bytes"));
        assert_eq!(
            explain(b"version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n"),
            Some("a Git LFS pointer, not the audio file")
        );
        // An Ogg page: `OggS`, 22 bytes of header fields, one segment, then the first packet.
        let mut opus = b"OggS".to_vec();
        opus.extend([0u8; 22]);
        opus.push(1);
        opus.push(19);
        opus.extend(b"OpusHead");
        assert!(explain(&opus).is_some_and(|hint| hint.contains("Opus")));
        let mut vorbis = opus.clone();
        vorbis.splice(28..36, *b"\x01vorbis\0");
        assert_eq!(explain(&vorbis), None, "an Ogg whose first packet is not OpusHead says nothing");
        assert_eq!(explain(b"OggS"), None, "and a cut file does not panic the sniff");
        assert_eq!(explain(b"RIFF....WAVE"), None);
    }

    /// 16-bit mono PCM, 8 kHz, 80 samples: a file `hound` reads in every build.
    fn wav() -> Vec<u8> {
        let data: Vec<u8> = (0..80i16).flat_map(|n| (n * 100).to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend(b"RIFF");
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(8000u32.to_le_bytes());
        out.extend(16_000u32.to_le_bytes());
        out.extend(2u16.to_le_bytes());
        out.extend(16u16.to_le_bytes());
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    /// **One decoder construction per sound, however often it is played and however many of the crate's systems ask** — including the voice that never starts here (a headless app has no device for Bevy to open), which the in-flight
    /// system asks about every frame. The memo is in `SirenSounds`, shared by `play_requests` and `refuse_unplayable_voices`; with one memo each, the first play of a Vorbis cost two constructions (0.8 ms each, release).
    #[test]
    fn a_sound_is_decoded_once_by_the_crate_however_often_it_is_played() {
        use bevy::asset::AssetPlugin;

        use crate::cue::{Cue, CueTable, SoundSlot};
        use crate::play::Play;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>()
            .add_plugins(crate::SirenPlugin);
        app.world_mut().remove_resource::<crate::output::Output>();
        app.insert_resource(CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("s")], ..Cue::new("c") }]));
        let handle = app.world_mut().resource_mut::<Assets<AudioSource>>().add(source(&wav()));
        app.world_mut().resource_mut::<SirenSounds>().insert("s", handle.clone());
        let address = {
            let assets = app.world().resource::<Assets<AudioSource>>();
            Arc::as_ptr(&assets.get(&handle).expect("the asset").bytes).cast::<u8>().addr()
        };
        let before = DECODED.lock().unwrap_or_else(PoisonError::into_inner).len();

        for _ in 0..4 {
            app.world_mut().write_message(Play::cue("c"));
            app.update();
        }

        let decodes = DECODED.lock().unwrap_or_else(PoisonError::into_inner)[before..]
            .iter()
            .filter(|made| **made == address)
            .count();
        assert_eq!(decodes, 1, "four plays, four frames of the in-flight system asking: one construction");
    }
}
