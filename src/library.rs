//! **A directory of audio, scanned into a manifest.**
//!
//! The cue table names the sounds a game plays; it says nothing about the thousands of files an asset library
//! holds that no cue names yet. Semantic selection needs the opposite direction — a walk of a directory that finds
//! every decodable file and gives each one a stable identity — and that identity is a **content hash**, not a path.
//!
//! Paths are not identity here, and this is the reason: an asset that is re-encoded, renamed or moved between
//! folders is the same sound, and re-embedding it would cost a GPU pass for nothing. A hash of the bytes survives
//! a rename and does not survive a re-encode, which is exactly the distinction an incremental index needs
//! ([`Library::added_since`] and [`Library::removed_since`]). Two files with the same bytes are one asset in two
//! places, and the manifest keeps both paths.
//!
//! Nothing here decodes audio: a scan reads a fixed prefix of every file and its header, so it costs a directory
//! walk and a few kilobytes per asset rather than a full decode. Decoding is `I-02`'s job.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The formats the crate can decode with either backend. A scan reports anything else as [`SkipReason::NotAudio`]
/// rather than silently ignoring it, so a library that is 90 % `.mp3` says so instead of indexing its `.wav` third.
///
/// **This list and `rodio`'s decoder features in `Cargo.toml` are one fact stated twice**, and they once disagreed: `mp3` and
/// `flac` were here and not there, so a folder of them scanned as audio and then failed to decode.
/// `tests/semantic_decode.rs::every_extension_the_scan_calls_audio_has_a_decoder` decodes a real file of each entry and fails if
/// an entry is added without a decoder or without a fixture.
pub const AUDIO_EXTENSIONS: &[&str] = &["wav", "ogg", "oga", "flac", "mp3"];

/// One file in the library, identified by its bytes.
///
/// `hash` is the 64-bit FNV-1a of the whole file. A 64-bit hash is chosen over a longer one because a library is
/// thousands of files, not billions, and a collision that matters would show up as a dropped asset in a scan — which
/// [`Library::duplicated`] reports rather than hides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// Every path this content appears at, relative to the scan root, sorted. One entry for a normal file; two or
    /// more when the same bytes were copied into two folders.
    pub paths: Vec<PathBuf>,
    /// The content hash: FNV-1a over the whole file.
    pub hash: u64,
    /// Size in bytes, so a manifest can be sanity-checked against the disk without reading every file again.
    pub bytes: u64,
    /// The sample rate read from the file header, when it could be read.
    pub sample_rate: Option<u32>,
    /// The channel count read from the file header, when it could be read.
    pub channels: Option<u16>,
}

/// A file that was found and not indexed, and why. A scan never drops a file silently: a library with a broken
/// `.ogg` must say so, because the alternative is a developer wondering why one footstep never appears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    /// Path relative to the scan root.
    pub path: PathBuf,
    /// Why it is not in the manifest.
    pub reason: SkipReason,
}

/// Why a file the scan found is not an asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// The extension is not one the crate decodes.
    NotAudio {
        /// The extension actually found, lowercased, without its dot.
        extension: String,
    },
    /// The file could not be opened or read.
    Unreadable {
        /// The operating system's message, passed through rather than paraphrased.
        error: String,
    },
    /// It is an audio extension, but the header does not identify a format the crate can decode.
    UndecodableHeader,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::NotAudio { extension } => {
                write!(f, "not an audio format this crate decodes (`.{extension}`)")
            }
            SkipReason::Unreadable { error } => write!(f, "unreadable: {error}"),
            SkipReason::UndecodableHeader => {
                write!(f, "audio extension but the header names no format it decodes")
            }
        }
    }
}

/// The result of a scan: every asset, every skip, and the bytes hashed to get there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    /// Assets keyed by content hash, so the identity is the content and the paths are data.
    pub assets: BTreeMap<u64, Asset>,
    /// Files that were found and not indexed.
    pub skipped: Vec<Skip>,
    /// Directories the walk refused to follow, with the reason. A symlink that leaves the root is refused rather
    /// than followed: a game's asset folder should not pull in a home directory.
    pub refused: Vec<(PathBuf, String)>,
}

impl Library {
    /// Every asset in path order, for a stable iteration order in a test or a report.
    pub fn by_path(&self) -> Vec<&Asset> {
        let mut all: Vec<&Asset> = self.assets.values().collect();
        all.sort_by(|a, b| a.paths.first().cmp(&b.paths.first()));
        all
    }

    /// How many files contributed bytes, counting an asset once however many paths it has.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// True when the scan found no audio at all.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// The content hashes that appear at more than one path. Not an error — a deliberate copy is legitimate — but
    /// it is reported, because an index that silently collapsed two folders would be surprising.
    pub fn duplicated(&self) -> Vec<&Asset> {
        self.assets.values().filter(|a| a.paths.len() > 1).collect()
    }

    /// What this scan has that `other` does not, by content hash: the adds an incremental index would embed.
    pub fn added_since(&self, other: &Library) -> Vec<&Asset> {
        self.assets.values().filter(|a| !other.assets.contains_key(&a.hash)).collect()
    }

    /// Content hashes in `other` that this scan no longer finds: the vectors an incremental index would drop.
    pub fn removed_since(&self, other: &Library) -> Vec<u64> {
        other.assets.keys().copied().filter(|h| !self.assets.contains_key(h)).collect()
    }
}

/// **Scan `root` for audio.**
///
/// The walk is breadth-first and sorted, so two scans of the same tree produce the same manifest in the same order.
/// It does not follow a symlink out of `root`; it does not descend into a directory it cannot read; and it records
/// both refusals in [`Library::refused`] rather than returning an error, because a library with one unreadable
/// folder is still a library.
pub fn scan(root: &Path) -> Library {
    let mut library = Library::default();
    let mut queue: Vec<PathBuf> = vec![root.to_path_buf()];
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    while let Some(dir) = queue.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                library.refused.push((relative(root, &dir), format!("cannot read directory: {e}")));
                continue;
            }
        };
        let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort();

        for path in children {
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    library.skipped.push(Skip {
                        path: relative(root, &path),
                        reason: SkipReason::Unreadable { error: e.to_string() },
                    });
                    continue;
                }
            };
            if meta.is_symlink() {
                // Follow only when it stays inside the root. A link out of the library is a refusal, not a skip:
                // the scan cannot know whether the target is audio without leaving the tree it was given.
                let target = path.canonicalize().unwrap_or_else(|_| path.clone());
                if target.starts_with(&canonical_root) {
                    if target.is_dir() {
                        queue.push(target);
                    } else if target.is_file() {
                        index_file(root, &target, &mut library);
                    }
                } else {
                    library
                        .refused
                        .push((relative(root, &path), "symlink leaves the library root".to_string()));
                }
                continue;
            }
            if meta.is_dir() {
                queue.push(path);
            } else if meta.is_file() {
                index_file(root, &path, &mut library);
            }
        }
    }

    library.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    library.refused.sort();
    library
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn extension_of(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn index_file(root: &Path, path: &Path, library: &mut Library) {
    let rel = relative(root, path);
    let ext = extension_of(path);
    if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        library.skipped.push(Skip { path: rel, reason: SkipReason::NotAudio { extension: ext } });
        return;
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            library.skipped.push(Skip { path: rel, reason: SkipReason::Unreadable { error: e.to_string() } });
            return;
        }
    };
    let (sample_rate, channels) = match read_header(&bytes, &ext) {
        Some(h) => h,
        None => {
            library.skipped.push(Skip { path: rel, reason: SkipReason::UndecodableHeader });
            return;
        }
    };
    let hash = fnv1a(&bytes);
    let len = bytes.len() as u64;
    match library.assets.get_mut(&hash) {
        Some(asset) => {
            if !asset.paths.contains(&rel) {
                asset.paths.push(rel);
                asset.paths.sort();
            }
        }
        None => {
            library.assets.insert(hash, Asset { paths: vec![rel], hash, bytes: len, sample_rate, channels });
        }
    }
}

/// **FNV-1a, 64-bit.** Chosen because it is eight lines, has no dependency, and is well behaved on short inputs —
/// which is what a library of sound effects is. A cryptographic hash would be slower and buy nothing: the threat
/// model is a developer copying files, not an adversary crafting a collision.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The sample rate and channel count from a file's header, or `None` when the header is not one this crate decodes.
///
/// Only the fields a scan needs are read. A full parse is `I-02`'s job, through the decoder that will actually decode
/// the file — this exists so a manifest can be inspected without a decoder in the build.
fn read_header(bytes: &[u8], extension: &str) -> Option<(Option<u32>, Option<u16>)> {
    // RIFF/WAVE: "RIFF" .... "WAVE", then a chunk walk to "fmt ".
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        let mut pos = 12usize;
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size =
                u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]) as usize;
            if id == b"fmt " && pos + 8 + 16 <= bytes.len() {
                let fmt = &bytes[pos + 8..];
                let channels = u16::from_le_bytes([fmt[2], fmt[3]]);
                let rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
                return Some((Some(rate), Some(channels)));
            }
            pos = pos.checked_add(8)?.checked_add(size)?.checked_add(size & 1)?;
        }
        // A RIFF container whose chunk walk ran off the end has no `fmt `, so nothing can decode it. That is a
        // corrupt file, not a WAV with an unknown rate, and saying so is the whole point of `Skip`.
        return None;
    }
    // Ogg: "OggS", then the first packet's identification header names the codec. Vorbis's (Vorbis I, §4.2.2) is, after its
    // seven-byte marker `\x01vorbis`: the version (4 bytes), the channel count (1), the sample rate (4, little-endian), and then
    // three bitrates. Read from `p`, the byte after the marker: channels at `p + 4`, rate at `p + 5 ..= p + 8`.
    if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        if let Some(i) = find(bytes, b"\x01vorbis") {
            let p = i + 7;
            if p + 9 <= bytes.len() {
                let channels = u16::from(bytes[p + 4]);
                let rate = u32::from_le_bytes([bytes[p + 5], bytes[p + 6], bytes[p + 7], bytes[p + 8]]);
                return Some((Some(rate), Some(channels)));
            }
        }
        return Some((None, None));
    }
    // FLAC: "fLaC", then the STREAMINFO metadata block (RFC 9639 §8.2): a four-byte block header, then 34 bytes that open with two
    // 16-bit block sizes and two 24-bit frame sizes — ten bytes — and continue with the sample rate in **20 bits**, the channel
    // count less one in **3**, and the bit depth less one in **5**. The rate therefore straddles three bytes: all of `s[10]`, all
    // of `s[11]`, and the high nibble of `s[12]`.
    if bytes.len() >= 4 && &bytes[0..4] == b"fLaC" {
        if bytes.len() >= 4 + 4 + 18 {
            let s = &bytes[8..];
            let rate = (u32::from(s[10]) << 12) | (u32::from(s[11]) << 4) | (u32::from(s[12]) >> 4);
            let channels = u16::from((s[12] >> 1) & 0x07) + 1;
            return Some((Some(rate), Some(channels)));
        }
        return Some((None, None));
    }
    // MP3 without a container: an ID3v2 tag or a frame sync. The rate needs a frame header parse, so only the
    // format is confirmed and the numbers are left unknown rather than guessed.
    if extension == "mp3" && (bytes.starts_with(b"ID3") || find(bytes, &[0xff, 0xfb]).is_some()) {
        return Some((None, None));
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
