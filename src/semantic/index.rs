//! **An index of embeddings, stamped with the model that made them.**
//!
//! An index is only valid for the model that produced it. Upgrade the model and every vector is a different space;
//! comparing them silently returns nonsense that looks like a ranking problem. So the stamp is written here, at the
//! only place an index is written, and [`Index::load`] refuses a mismatch rather than ranking across two spaces.
//!
//! # The format, and why it is this one
//!
//! A text file with a header and one record per asset. It is chosen because:
//!
//! - **It can be read by a person**, which matters more than speed for a file whose corruption would otherwise be
//!   invisible: `grep` an asset id and see its vector.
//! - **It has no dependency.** The crate's list is closed (`tests/leaf.rs`); `S-04` may choose SQLite later, and this
//!   format is what the tests prove the *rules* with in the meantime.
//! - **Floats are written in a fixed, round-trip-exact form.** `f32` formatted with Rust's shortest round-trip
//!   representation and parsed back is bit-identical, so the round-trip test is a real assertion rather than a
//!   tolerance.
//!
//! # Truncation
//!
//! A write interrupted halfway leaves a file with fewer records than its header claims. The header carries the record
//! count and a checksum of the body, so [`Index::load`] **detects** a truncated or corrupted file instead of reading
//! part of it as if it were whole.

use std::collections::BTreeMap;

use super::loudness::Loudness;
use super::provenance::Provenance;
use super::spectral::Spectral;
use super::tempo::Tempo;
use std::fmt::Write as _;

/// The identity of the model that made an index's vectors.
///
/// Everything that would change the meaning of a vector belongs here: a different model, a different version of the
/// same model, or a different input rate all produce vectors that cannot be compared with the ones on disk.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelStamp {
    /// The model's name as its card spells it, e.g. `laion/clap-htsat-fused`.
    pub model: String,
    /// The version or revision, e.g. a commit sha or a release tag.
    pub version: String,
    /// The sample rate the audio was resampled to before embedding.
    pub sample_rate: u32,
    /// The length of one vector, so a truncated vector is caught at parse time.
    pub dimension: usize,
}

impl ModelStamp {
    /// A stamp for CLAP as it was measured: `laion/clap-htsat-fused`, 48 kHz, 512 dimensions.
    ///
    /// The version names the measurement rather than a tag, because the spike did not pin a revision: it is
    /// [`crate::semantic::MEASURED_CLAP_VERSION`], and an index stamped with it is an index this crate has evidence for.
    pub fn measured_clap() -> Self {
        ModelStamp {
            model: "laion/clap-htsat-fused".to_string(),
            version: crate::semantic::MEASURED_CLAP_VERSION.to_string(),
            sample_rate: crate::semantic::MODEL_SAMPLE_RATE,
            dimension: crate::semantic::EMBEDDING_DIM,
        }
    }
}

impl core::fmt::Display for ModelStamp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}@{} {}Hz x{}", self.model, self.version, self.sample_rate, self.dimension)
    }
}

/// Why an index could not be loaded.
///
/// A mismatch names **both** sides, because the message is read by someone deciding whether to re-index or to find the
/// other file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexProblem {
    /// The file's header is missing or malformed.
    Header {
        /// The line that could not be read, quoted so a person can find it.
        line: String,
    },
    /// The index was built by a different model than the one loaded.
    ///
    /// Boxed because two stamps are far larger than this enum needs to be: a `Result` carrying it would be padded on
    /// every call, including the successful ones.
    ModelMismatch(Box<Mismatch>),
    /// The header promised more records than the body holds, which is what an interrupted write looks like.
    Truncated {
        /// Records the header claims.
        expected: usize,
        /// Records actually present.
        found: usize,
    },
    /// The body does not match the header's checksum.
    Checksum {
        /// What the header says.
        expected: u64,
        /// What the body hashes to.
        found: u64,
    },
    /// The bytes are not an index this crate can read, carrying what was found where.
    Bytes {
        /// What the reader expected or what it found, so the message points at the bytes.
        why: String,
    },
    /// A record is not the shape the header promised.
    Record {
        /// 1-based line number, so the message points at a line a person can open. Zero when the record was rejected
        /// before it was written, as an insert of the wrong length is.
        line: usize,
        /// What was wrong with it.
        what: String,
    },
}

impl core::fmt::Display for IndexProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IndexProblem::Header { line } => write!(f, "unreadable index header: {line}"),
            IndexProblem::ModelMismatch(m) => write!(
                f,
                "this index was built by {} and {} is loaded. Re-index the library, or set the model \
                 version to the index's.",
                m.index, m.loaded
            ),
            IndexProblem::Truncated { expected, found } => write!(
                f,
                "the index is truncated: its header claims {expected} records and {found} are present. A write was \
                 interrupted; rebuild the index."
            ),
            IndexProblem::Checksum { expected, found } => {
                write!(
                    f,
                    "the index body does not match its checksum ({expected:#x} vs {found:#x}); it is corrupt"
                )
            }
            IndexProblem::Bytes { why } => write!(f, "this is not a binary index: {why}"),
            IndexProblem::Record { line, what } => write!(f, "index line {line}: {what}"),
        }
    }
}

/// One asset's vector, keyed by the content hash [`crate::library`] gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The asset's content hash: the identity a rename does not change.
    pub hash: u64,
    /// The embedding, `dimension` long.
    pub vector: Vec<f32>,
}

/// **An index: a stamp, one vector per content hash, and — optionally — where each asset came from.**
///
/// `provenance` is a **trailing optional section**, per `needs-decision/provenance-in-the-index.md`: an index written
/// before it existed loads unchanged, and an asset with no record is simply absent rather than carrying a sentinel.
/// Absence is absence, which is the same rule `#21` learned about a short clip's loudness.
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    /// The model that made every vector here.
    pub stamp: ModelStamp,
    /// Vectors by content hash.
    pub records: BTreeMap<u64, Vec<f32>>,
    /// Where each asset came from, by content hash. Empty when nothing was recorded — never a map of unknowns.
    pub provenance: BTreeMap<u64, Provenance>,
    /// Each asset's loudness, by content hash — the same optional-section shape as `provenance`, per
    /// `needs-decision/loudness-in-the-index.md`.
    pub loudness: BTreeMap<u64, Loudness>,
    /// Each asset's spectral shape, by content hash — a third optional section, same rules.
    pub spectral: BTreeMap<u64, Spectral>,
    /// Each asset's tempo, by content hash. **Absent for a non-rhythmic asset**, which is a fact worth storing: a
    /// stored `None` would be indistinguishable from "not yet measured", and the filter needs to tell them apart.
    pub tempo: BTreeMap<u64, Tempo>,
    /// Each asset's key **and its confidence**, by content hash. Absent for an atonal asset, for the same reason.
    pub key: BTreeMap<u64, super::key::Estimate>,
}

const MAGIC: &str = "bevy_siren index 1";

/// **A field as one whitespace-free token**, so a licence with a space in it survives a space-delimited format.
///
/// `None` becomes `-`, and a value that is genuinely `-` or contains a space or a backslash is escaped with `\`. The
/// escape is what keeps *recorded absence* distinguishable from *recorded content that looks like absence* — the same
/// distinction `Provenance` draws in memory.
fn one_line(value: Option<&str>) -> String {
    match value {
        None => String::from("-"),
        Some(v) => escape(v),
    }
}

/// Escape a field for the space-delimited provenance section.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            ' ' => out.push_str("\\s"),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' => out.push_str("\\n"),
            '-' if value == "-" => out.push_str("\\-"),
            c => out.push(c),
        }
    }
    if out.is_empty() { String::from("\\e") } else { out }
}

/// Unescape a field written by [`escape`], or `None` for the recorded-absence marker.
fn unescape(field: &str) -> Option<String> {
    if field == "-" {
        return None;
    }
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some('-') => out.push('-'),
            Some('e') => {}
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    Some(out)
}

/// **An integrated loudness as one field**, or `-` when the clip is too short to measure.
///
/// `-` rather than a sentinel number is the entire reason this section exists in this shape: ADR 0003 measured that
/// `ffmpeg` reports `-70.0 LUFS` and `ebur128` reports `-inf` for a 0.2 s clip, and both are wrong because both are
/// *numbers*. `-70.0` is also a real level, so a sentinel would erase the difference between "too short" and "quiet".
fn lufs_field(value: Option<f32>) -> String {
    match value {
        // A non-finite value cannot round-trip through text, and reporting it as a measurement would be a lie.
        Some(v) if v.is_finite() => format!("{v:?}"),
        _ => String::from("-"),
    }
}

/// A dB level as one field. A `f32` that is not finite is written `-`, because the crate's own rule is that a number
/// which is not a number is silence and must not be presented as a measurement.
fn db_field(value: f32) -> String {
    if value.is_finite() { format!("{value:?}") } else { String::from("-") }
}

/// A frequency field, or `-` when silence had no centroid to report.
fn hertz_field(value: Option<f32>) -> String {
    match value {
        Some(v) if v.is_finite() => format!("{v:?}"),
        _ => String::from("-"),
    }
}

/// One tempo record: `<16 hex hash> <bpm|-> <confidence> <octave-ambiguous 0|1>`.
fn parse_tempo_line(line: &str) -> Option<(u64, super::tempo::Tempo)> {
    let mut f = line.split(' ');
    let hash = u64::from_str_radix(f.next()?, 16).ok()?;
    let bpm = match f.next()? {
        "-" => None,
        text => Some(text.parse::<f32>().ok().filter(|v| v.is_finite())?),
    };
    let confidence = f.next()?.parse::<f32>().ok()?;
    let octave_ambiguous = match f.next()? {
        "1" => true,
        "0" => false,
        _ => return None,
    };
    Some((hash, super::tempo::Tempo { bpm, confidence, octave_ambiguous }))
}

/// One key record: `<16 hex hash> <tonic|-> <major 0|1> <confidence>`.
fn parse_key_line(line: &str) -> Option<(u64, super::key::Estimate)> {
    let mut f = line.split(' ');
    let hash = u64::from_str_radix(f.next()?, 16).ok()?;
    let tonic = match f.next()? {
        "-" => None,
        text => Some(text.parse::<f32>().ok()?.round() as u8 % 12),
    };
    let major = match f.next()? {
        "1" => true,
        "0" => false,
        _ => return None,
    };
    let confidence = f.next()?.parse::<f32>().ok()?;
    Some((hash, super::key::Estimate { key: super::key::Key { tonic, major }, confidence }))
}

/// One spectral record: `<16 hex hash> <centroid|-> <rolloff|-> <noise floor dB>`.
fn parse_spectral_line(line: &str) -> Option<(u64, Spectral)> {
    let mut fields = line.split(' ');
    let hash = u64::from_str_radix(fields.next()?, 16).ok()?;
    let hertz = |f: &str| if f == "-" { Some(None) } else { f.parse::<f32>().ok().map(Some) };
    let centroid_hz = hertz(fields.next()?)?;
    let rolloff_hz = hertz(fields.next()?)?;
    let noise_floor_db = fields.next()?.parse::<f32>().ok()?;
    Some((hash, Spectral { centroid_hz, rolloff_hz, noise_floor_db }))
}

/// One loudness record: `<16 hex hash> <lufs|-> <dbtp> <sample peak> <blocks> <blocks_possible>`.
fn parse_loudness_line(line: &str) -> Option<(u64, Loudness)> {
    let mut fields = line.split(' ');
    let hash = u64::from_str_radix(fields.next()?, 16).ok()?;
    let integrated = match fields.next()? {
        "-" => None,
        text => Some(text.parse::<f32>().ok()?),
    };
    let true_peak = fields.next()?.parse::<f32>().ok()?;
    let sample_peak = fields.next()?.parse::<f32>().ok()?;
    let blocks = fields.next()?.parse::<u64>().ok()?;
    let blocks_possible = fields.next()?.parse::<u64>().ok()?;
    Some((hash, Loudness { integrated, true_peak, sample_peak, blocks, blocks_possible }))
}

/// One provenance record: `<16 hex hash> <licence> <author> <source>`, or `None` when the line names no hash.
fn parse_provenance_line(line: &str) -> Option<(u64, Provenance)> {
    let mut fields = line.split(' ');
    let hash = u64::from_str_radix(fields.next()?, 16).ok()?;
    let licence = unescape(fields.next().unwrap_or("-"));
    let author = unescape(fields.next().unwrap_or("-"));
    let source = match fields.next().unwrap_or("-") {
        "-" => None,
        "?unknown" => Some(super::provenance::Source::Unknown),
        token => {
            let (kind, rest) = token.split_at(1);
            let value = unescape(rest).unwrap_or_default();
            match kind {
                "u" => Some(super::provenance::Source::Url(value)),
                "p" => Some(super::provenance::Source::Pack(value)),
                // An unknown kind is a record this crate cannot interpret. Refusing the whole line keeps the store
                // honest: a silently-dropped source would present a recorded origin as unrecorded.
                _ => return None,
            }
        }
    };
    Some((hash, Provenance { licence, source, author }))
}

/// **The binary format's magic**, distinct from the text one so a file identifies itself without reading its extension.
///
/// ADR 0004 chose this: the text form stays as the *debuggable* form and the binary form is what a 5 000-clip library
/// ships, and the two are told apart by their first four bytes rather than by their names, because a name is not
/// evidence.
pub const MAGIC_BINARY: &[u8; 4] = b"BSI2";

/// The two stamps in a [`IndexProblem::ModelMismatch`], boxed as one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// What the file says.
    pub index: ModelStamp,
    /// What is loaded now.
    pub loaded: ModelStamp,
}

impl Index {
    /// A new, empty index for `stamp`.
    pub fn new(stamp: ModelStamp) -> Self {
        Index {
            stamp,
            records: BTreeMap::new(),
            provenance: BTreeMap::new(),
            loudness: BTreeMap::new(),
            spectral: BTreeMap::new(),
            tempo: BTreeMap::new(),
            key: BTreeMap::new(),
        }
    }

    /// How many assets are indexed.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Insert or replace one vector. Refused when it is the wrong length, because a short vector would compare
    /// against a full one and produce a score from nothing.
    pub fn insert(&mut self, hash: u64, vector: Vec<f32>) -> Result<(), IndexProblem> {
        if vector.len() != self.stamp.dimension {
            return Err(IndexProblem::Record {
                line: 0,
                what: format!(
                    "a {}-element vector cannot enter an index of {}-element vectors",
                    vector.len(),
                    self.stamp.dimension
                ),
            });
        }
        self.records.insert(hash, vector);
        Ok(())
    }

    /// **Write the index**, in the format described in this module's documentation.
    ///
    /// The header carries the record count and a checksum of the body, so [`Index::load`] can tell a truncated file
    /// from a whole one without a separate length file beside it.
    pub fn to_text(&self) -> String {
        let mut body = String::new();
        for (hash, vector) in &self.records {
            let _ = write!(body, "{hash:016x}");
            for v in vector {
                // `{:?}` on an f32 is Rust's shortest round-trip form: parsing it back gives the same bits.
                let _ = write!(body, " {v:?}");
            }
            body.push('\n');
        }
        // **The provenance section is part of the checksummed body**, so a record cannot be edited without the file
        // being detected as corrupt. It is written only when there is something to write: an index with no provenance
        // is byte-identical to one written before this section existed, which is what makes the change additive.
        if !self.provenance.is_empty() {
            let _ = writeln!(body, "provenance {}", self.provenance.len());
            for (hash, p) in &self.provenance {
                let _ = write!(body, "{hash:016x}");
                // `-` is the recorded absence in one column. A licence that is genuinely the single character `-`
                // would be written `\-`, so the escape keeps absence and content distinguishable.
                let _ = write!(body, " {}", one_line(p.licence.as_deref()));
                let _ = write!(body, " {}", one_line(p.author.as_deref()));
                let source = match &p.source {
                    None => String::from("-"),
                    Some(super::provenance::Source::Unknown) => String::from("?unknown"),
                    Some(super::provenance::Source::Url(u)) => format!("u{}", escape(u)),
                    Some(super::provenance::Source::Pack(p)) => format!("p{}", escape(p)),
                };
                let _ = writeln!(body, " {source}");
            }
        }
        // **The loudness section, same rules.** Inside the checksummed body, written only when non-empty, and
        // absence encoded as `-` rather than as a number — the whole point of ADR 0003's short-clip finding.
        if !self.loudness.is_empty() {
            let _ = writeln!(body, "loudness {}", self.loudness.len());
            for (hash, l) in &self.loudness {
                let _ = write!(body, "{hash:016x} {}", lufs_field(l.integrated));
                let _ = write!(body, " {}", db_field(l.true_peak));
                let _ = write!(body, " {}", db_field(l.sample_peak));
                let _ = writeln!(body, " {} {}", l.blocks, l.blocks_possible);
            }
        }
        // **A third optional section, same rules.** `-` for an absent centroid or rolloff: silence has no brightness,
        // and writing `0` there would make it indistinguishable from a DC offset.
        if !self.spectral.is_empty() {
            let _ = writeln!(body, "spectral {}", self.spectral.len());
            for (hash, s) in &self.spectral {
                let _ = write!(body, "{hash:016x} {}", hertz_field(s.centroid_hz));
                let _ = write!(body, " {}", hertz_field(s.rolloff_hz));
                let _ = writeln!(body, " {}", db_field(s.noise_floor_db));
            }
        }
        // **Tempo**: `<hash> <bpm|-> <confidence> <octave-ambiguous 0|1>`. A `-` bpm is *no tempo*, which is why it is
        // written rather than omitted — a stored `None` and an unmeasured asset are different facts, and a filter reads
        // the first as *unjudged*.
        if !self.tempo.is_empty() {
            let _ = writeln!(body, "tempo {}", self.tempo.len());
            for (hash, t) in &self.tempo {
                let _ = write!(body, "{hash:016x} {}", hertz_field(t.bpm));
                let _ = writeln!(body, " {} {}", db_field(t.confidence), u8::from(t.octave_ambiguous));
            }
        }
        // **Key**: `<hash> <tonic|-> <major 0|1> <confidence>`. A `-` tonic is *no key*, from the same reasoning.
        if !self.key.is_empty() {
            let _ = writeln!(body, "key {}", self.key.len());
            for (hash, k) in &self.key {
                let _ = write!(body, "{hash:016x} {}", hertz_field(k.key.tonic.map(f32::from)));
                let _ = writeln!(body, " {} {}", u8::from(k.key.major), db_field(k.confidence));
            }
        }
        let checksum = crate::library::fnv1a(body.as_bytes());
        let mut out = String::new();
        let _ = writeln!(out, "{MAGIC}");
        let _ = writeln!(out, "model {}", self.stamp.model);
        let _ = writeln!(out, "version {}", self.stamp.version);
        let _ = writeln!(out, "sample_rate {}", self.stamp.sample_rate);
        let _ = writeln!(out, "dimension {}", self.stamp.dimension);
        let _ = writeln!(out, "records {}", self.records.len());
        let _ = writeln!(out, "checksum {checksum:016x}");
        out.push_str(&body);
        out
    }

    /// **Read an index back**, refusing a model mismatch, a truncated file and a corrupt one.
    ///
    /// `loaded` is the stamp of the model currently running. Passing it is not optional and not a default: it is the
    /// whole reason the stamp exists, and a caller that does not know which model it has cannot safely rank anything.
    pub fn load(text: &str, loaded: &ModelStamp) -> Result<Index, IndexProblem> {
        let mut lines = text.lines();
        let first =
            lines.next().ok_or_else(|| IndexProblem::Header { line: String::from("<empty file>") })?;
        if first.trim() != MAGIC {
            return Err(IndexProblem::Header { line: first.to_string() });
        }

        let mut stamp =
            ModelStamp { model: String::new(), version: String::new(), sample_rate: 0, dimension: 0 };
        let mut expected_records: Option<usize> = None;
        let mut expected_checksum: Option<u64> = None;

        for _ in 0..6 {
            let line = lines
                .next()
                .ok_or_else(|| IndexProblem::Header { line: String::from("header ends early") })?;
            let (key, value) =
                line.split_once(' ').ok_or_else(|| IndexProblem::Header { line: line.to_string() })?;
            let bad = || IndexProblem::Header { line: line.to_string() };
            match key {
                "model" => stamp.model = value.to_string(),
                "version" => stamp.version = value.to_string(),
                "sample_rate" => stamp.sample_rate = value.parse().map_err(|_| bad())?,
                "dimension" => stamp.dimension = value.parse().map_err(|_| bad())?,
                "records" => expected_records = Some(value.parse().map_err(|_| bad())?),
                "checksum" => {
                    expected_checksum = Some(u64::from_str_radix(value, 16).map_err(|_| bad())?);
                }
                _ => return Err(bad()),
            }
        }

        let body_start = text
            .find("\n")
            .and_then(|_| text.match_indices('\n').nth(6).map(|(i, _)| i + 1))
            .ok_or_else(|| IndexProblem::Header { line: String::from("header ends early") })?;
        let body = &text[body_start..];

        let expected_records =
            expected_records.ok_or_else(|| IndexProblem::Header { line: String::from("no record count") })?;
        let expected_checksum =
            expected_checksum.ok_or_else(|| IndexProblem::Header { line: String::from("no checksum") })?;
        let found_checksum = crate::library::fnv1a(body.as_bytes());
        if found_checksum != expected_checksum {
            return Err(IndexProblem::Checksum { expected: expected_checksum, found: found_checksum });
        }

        if &stamp != loaded {
            return Err(IndexProblem::ModelMismatch(Box::new(Mismatch {
                index: stamp,
                loaded: loaded.clone(),
            })));
        }

        let mut records = BTreeMap::new();
        let mut provenance: BTreeMap<u64, Provenance> = BTreeMap::new();
        let mut loudness: BTreeMap<u64, Loudness> = BTreeMap::new();
        let mut expected_provenance: Option<usize> = None;
        let mut expected_loudness: Option<usize> = None;
        let mut spectral: BTreeMap<u64, Spectral> = BTreeMap::new();
        let mut expected_spectral: Option<usize> = None;
        let mut tempo: BTreeMap<u64, super::tempo::Tempo> = BTreeMap::new();
        let mut expected_tempo: Option<usize> = None;
        let mut key: BTreeMap<u64, super::key::Estimate> = BTreeMap::new();
        let mut expected_key: Option<usize> = None;
        // **A section marker is the only line that can start with `provenance ` or `loudness `**, because every record
        // line begins with a 16-hex-character hash. Each section runs until the next marker or the end of the body, so
        // several can coexist and their order does not matter — except for their own records, which must follow them.
        #[derive(Clone, Copy, PartialEq)]
        enum Section {
            Vectors,
            Provenance,
            Loudness,
            Spectral,
            Tempo,
            Key,
        }
        let mut section = Section::Vectors;
        for (n, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let line_no = n + 8;
            // **Markers are recognised before anything else**, whatever section is current. A `loudness 1` line that
            // arrives while the provenance section is open is the *next section*, not a broken provenance record — a
            // distinction the first version of this parser got wrong, and `provenance_and_loudness_sections_coexist`
            // caught it as `a provenance record names no hash` on the marker line.
            if let Some(count) = line.strip_prefix("provenance ") {
                expected_provenance = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a provenance record count"),
                })?);
                section = Section::Provenance;
                continue;
            }
            if let Some(count) = line.strip_prefix("loudness ") {
                expected_loudness = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a loudness record count"),
                })?);
                section = Section::Loudness;
                continue;
            }
            if section == Section::Provenance {
                if let Some((hash, record)) = parse_provenance_line(line) {
                    provenance.insert(hash, record);
                } else {
                    return Err(IndexProblem::Record {
                        line: line_no,
                        what: String::from("a provenance record names no hash"),
                    });
                }
                continue;
            }
            if let Some(count) = line.strip_prefix("spectral ") {
                expected_spectral = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a spectral record count"),
                })?);
                section = Section::Spectral;
                continue;
            }
            if let Some(count) = line.strip_prefix("tempo ") {
                expected_tempo = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a tempo record count"),
                })?);
                section = Section::Tempo;
                continue;
            }
            if let Some(count) = line.strip_prefix("key ") {
                expected_key = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a key record count"),
                })?);
                section = Section::Key;
                continue;
            }
            if section == Section::Tempo {
                if let Some((hash, record)) = parse_tempo_line(line) {
                    tempo.insert(hash, record);
                } else {
                    return Err(IndexProblem::Record {
                        line: line_no,
                        what: String::from(
                            "a tempo record is not `<hash> <bpm|-> <confidence> <octave 0|1>`",
                        ),
                    });
                }
                continue;
            }
            if section == Section::Key {
                if let Some((hash, record)) = parse_key_line(line) {
                    key.insert(hash, record);
                } else {
                    return Err(IndexProblem::Record {
                        line: line_no,
                        what: String::from("a key record is not `<hash> <tonic|-> <major 0|1> <confidence>`"),
                    });
                }
                continue;
            }
            if section == Section::Spectral {
                if let Some((hash, record)) = parse_spectral_line(line) {
                    spectral.insert(hash, record);
                } else {
                    return Err(IndexProblem::Record {
                        line: line_no,
                        what: String::from(
                            "a spectral record is not `<hash> <centroid|-> <rolloff|-> <floor dB>`",
                        ),
                    });
                }
                continue;
            }
            if section == Section::Loudness {
                if let Some((hash, record)) = parse_loudness_line(line) {
                    loudness.insert(hash, record);
                } else {
                    return Err(IndexProblem::Record {
                        line: line_no,
                        what: String::from(
                            "a loudness record is not \
                             `<hash> <lufs|-> <dbtp> <sample peak> <blocks> <blocks possible>`",
                        ),
                    });
                }
                continue;
            }
            let mut fields = line.split(' ');
            let hash_text = fields
                .next()
                .ok_or_else(|| IndexProblem::Record { line: line_no, what: String::from("no hash") })?;
            let hash = u64::from_str_radix(hash_text, 16).map_err(|_| IndexProblem::Record {
                line: line_no,
                what: format!("`{hash_text}` is not a hash"),
            })?;
            let mut vector = Vec::with_capacity(stamp.dimension);
            for f in fields {
                vector.push(f.parse::<f32>().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{f}` is not a number"),
                })?);
            }
            if vector.len() != stamp.dimension {
                return Err(IndexProblem::Record {
                    line: line_no,
                    what: format!("{} values, the header promises {}", vector.len(), stamp.dimension),
                });
            }
            records.insert(hash, vector);
        }

        if records.len() != expected_records {
            return Err(IndexProblem::Truncated { expected: expected_records, found: records.len() });
        }
        // A section that promises more records than it carries is a truncated file, the same failure the vector count
        // catches — and an index with no section at all has `None` here and is simply valid.
        if let Some(expected) = expected_provenance
            && provenance.len() != expected
        {
            return Err(IndexProblem::Truncated { expected, found: provenance.len() });
        }
        if let Some(expected) = expected_loudness
            && loudness.len() != expected
        {
            return Err(IndexProblem::Truncated { expected, found: loudness.len() });
        }
        if let Some(expected) = expected_spectral
            && spectral.len() != expected
        {
            return Err(IndexProblem::Truncated { expected, found: spectral.len() });
        }
        if let Some(expected) = expected_tempo
            && tempo.len() != expected
        {
            return Err(IndexProblem::Truncated { expected, found: tempo.len() });
        }
        if let Some(expected) = expected_key
            && key.len() != expected
        {
            return Err(IndexProblem::Truncated { expected, found: key.len() });
        }

        Ok(Index { stamp, records, provenance, loudness, spectral, tempo, key })
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`IndexProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for IndexProblem {}

impl Index {
    /// **Write the index as bytes**, the form ADR 0004 measured at **10.30 MB** for 5 000 clips against the text
    /// form's **31.88 MB**.
    ///
    /// Layout, little-endian throughout:
    ///
    /// | bytes | field |
    /// |---|---|
    /// | 4 | `MAGIC_BINARY` |
    /// | 4 | dimension, `u32` |
    /// | 4 | model-name length, `u32`, then that many UTF-8 bytes |
    /// | 4 | version length, `u32`, then that many UTF-8 bytes |
    /// | 4 | sample rate, `u32` |
    /// | 8 | record count, `u64` |
    /// | per record | `u64` hash, then `dimension` × `f32` |
    /// | 8 | provenance count, `u64`, then per entry `u64` hash + length-prefixed licence, author and source |
    /// | 8 | loudness count, `u64`, then per entry `u64` hash + `f32` lufs + a presence byte + `2 × f32` + `2 × u64` |
    /// | 8 | **FNV-1a of every preceding byte**, so a truncated file is detected |
    ///
    /// The checksum over the whole file is why the two trailing sections can be appended without a version bump: an
    /// older reader stops at its checksum, and a newer one that finds fewer bytes than the count promises refuses.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.records.len() * (8 + self.stamp.dimension * 4) + 2_048);
        out.extend_from_slice(MAGIC_BINARY);
        out.extend_from_slice(&(self.stamp.dimension as u32).to_le_bytes());
        push_str(&mut out, &self.stamp.model);
        push_str(&mut out, &self.stamp.version);
        out.extend_from_slice(&self.stamp.sample_rate.to_le_bytes());
        out.extend_from_slice(&(self.records.len() as u64).to_le_bytes());
        for (hash, vector) in &self.records {
            out.extend_from_slice(&hash.to_le_bytes());
            for v in vector {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out.extend_from_slice(&(self.provenance.len() as u64).to_le_bytes());
        for (hash, p) in &self.provenance {
            out.extend_from_slice(&hash.to_le_bytes());
            push_opt_str(&mut out, p.licence.as_deref());
            push_opt_str(&mut out, p.author.as_deref());
            match &p.source {
                None => out.push(b'-'),
                Some(super::provenance::Source::Unknown) => out.push(b'?'),
                Some(super::provenance::Source::Url(u)) => {
                    out.push(b'u');
                    push_str(&mut out, u);
                }
                Some(super::provenance::Source::Pack(p)) => {
                    out.push(b'p');
                    push_str(&mut out, p);
                }
            }
        }
        out.extend_from_slice(&(self.loudness.len() as u64).to_le_bytes());
        for (hash, l) in &self.loudness {
            out.extend_from_slice(&hash.to_le_bytes());
            // The presence byte is the whole reason this is not a bare `f32`: a short clip has NO integrated loudness,
            // and writing a sentinel there is what ADR 0003 measured three implementations getting wrong.
            match l.integrated {
                Some(v) => {
                    out.push(1);
                    out.extend_from_slice(&v.to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&0.0f32.to_le_bytes());
                }
            }
            out.extend_from_slice(&l.true_peak.to_le_bytes());
            out.extend_from_slice(&l.sample_peak.to_le_bytes());
            out.extend_from_slice(&l.blocks.to_le_bytes());
            out.extend_from_slice(&l.blocks_possible.to_le_bytes());
        }
        out.extend_from_slice(&(self.spectral.len() as u64).to_le_bytes());
        for (hash, s) in &self.spectral {
            out.extend_from_slice(&hash.to_le_bytes());
            // A presence byte per optional frequency, for the same reason loudness has one: `0` Hz is a real reading
            // for a DC offset, so a sentinel would conflate it with silence.
            match s.centroid_hz {
                Some(v) => {
                    out.push(1);
                    out.extend_from_slice(&v.to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&0.0f32.to_le_bytes());
                }
            }
            match s.rolloff_hz {
                Some(v) => {
                    out.push(1);
                    out.extend_from_slice(&v.to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&0.0f32.to_le_bytes());
                }
            }
            out.extend_from_slice(&s.noise_floor_db.to_le_bytes());
        }
        // **Tempo**: presence byte, `f32` bpm, `f32` confidence, octave byte.
        out.extend_from_slice(&(self.tempo.len() as u64).to_le_bytes());
        for (hash, t) in &self.tempo {
            out.extend_from_slice(&hash.to_le_bytes());
            match t.bpm {
                Some(v) => {
                    out.push(1);
                    out.extend_from_slice(&v.to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&0.0f32.to_le_bytes());
                }
            }
            out.extend_from_slice(&t.confidence.to_le_bytes());
            out.push(u8::from(t.octave_ambiguous));
        }
        // **Key**: presence byte, `f32` tonic, major byte, `f32` confidence.
        out.extend_from_slice(&(self.key.len() as u64).to_le_bytes());
        for (hash, k) in &self.key {
            out.extend_from_slice(&hash.to_le_bytes());
            match k.key.tonic {
                Some(t) => {
                    out.push(1);
                    out.extend_from_slice(&f32::from(t).to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&0.0f32.to_le_bytes());
                }
            }
            out.push(u8::from(k.key.major));
            out.extend_from_slice(&k.confidence.to_le_bytes());
        }
        let checksum = crate::library::fnv1a(&out);
        out.extend_from_slice(&checksum.to_le_bytes());
        out
    }

    /// **Read a binary index back**, refusing a foreign magic, a short file, a bad checksum and a model mismatch.
    ///
    /// Every read is bounds-checked: a truncated file must produce a *sentence*, not a panic, because the bytes came
    /// from disk and a half-written file is the ordinary failure.
    pub fn from_bytes(bytes: &[u8], loaded: &ModelStamp) -> Result<Index, IndexProblem> {
        let bad = |why: &str| IndexProblem::Bytes { why: why.to_string() };
        if bytes.len() < 12 {
            return Err(bad("the file is too short to hold a header"));
        }
        if &bytes[..4] != MAGIC_BINARY {
            return Err(bad("its first four bytes are not `BSI2`"));
        }
        // The checksum covers everything but itself, so it is checked first: a corrupt file should say so rather than
        // fail later on a length that happens to be wrong.
        if bytes.len() < 8 {
            return Err(bad("the file has no checksum"));
        }
        let (body, tail) = bytes.split_at(bytes.len() - 8);
        let expected = u64::from_le_bytes(tail.try_into().map_err(|_| bad("the checksum is not 8 bytes"))?);
        let found = crate::library::fnv1a(body);
        if found != expected {
            return Err(IndexProblem::Checksum { expected, found });
        }

        let mut at = 4usize;
        let dimension = read_u32(body, &mut at).map_err(|e| bad(&e))? as usize;
        let model = read_str(body, &mut at).map_err(|e| bad(&e))?;
        let version = read_str(body, &mut at).map_err(|e| bad(&e))?;
        let sample_rate = read_u32(body, &mut at).map_err(|e| bad(&e))?;
        let stamp = ModelStamp { model, version, sample_rate, dimension };
        if &stamp != loaded {
            return Err(IndexProblem::ModelMismatch(Box::new(Mismatch {
                index: stamp,
                loaded: loaded.clone(),
            })));
        }

        let count = read_u64(body, &mut at).map_err(|e| bad(&e))? as usize;
        let mut records = BTreeMap::new();
        for _ in 0..count {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let mut vector = Vec::with_capacity(dimension);
            for _ in 0..dimension {
                vector.push(read_f32(body, &mut at).map_err(|e| bad(&e))?);
            }
            records.insert(hash, vector);
        }

        let mut provenance: BTreeMap<u64, Provenance> = BTreeMap::new();
        for _ in 0..read_u64(body, &mut at).map_err(|e| bad(&e))? {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let licence = read_opt_str(body, &mut at).map_err(|e| bad(&e))?;
            let author = read_opt_str(body, &mut at).map_err(|e| bad(&e))?;
            let kind = *body.get(at).ok_or_else(|| bad("a provenance record ends early"))?;
            at += 1;
            let source = match kind {
                b'-' => None,
                b'?' => Some(super::provenance::Source::Unknown),
                b'u' => Some(super::provenance::Source::Url(read_str(body, &mut at).map_err(|e| bad(&e))?)),
                b'p' => Some(super::provenance::Source::Pack(read_str(body, &mut at).map_err(|e| bad(&e))?)),
                other => return Err(bad(&format!("unknown provenance source kind `{}`", other as char))),
            };
            provenance.insert(hash, Provenance { licence, source, author });
        }

        let mut loudness: BTreeMap<u64, super::loudness::Loudness> = BTreeMap::new();
        for _ in 0..read_u64(body, &mut at).map_err(|e| bad(&e))? {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let has = *body.get(at).ok_or_else(|| bad("a loudness record ends early"))?;
            at += 1;
            let value = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let integrated = if has == 1 {
                if !value.is_finite() {
                    return Err(bad("a loudness record says it has a value and carries a non-finite one"));
                }
                Some(value)
            } else {
                None
            };
            let true_peak = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let sample_peak = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let blocks = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let blocks_possible = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            loudness.insert(
                hash,
                super::loudness::Loudness { integrated, true_peak, sample_peak, blocks, blocks_possible },
            );
        }

        let mut spectral: BTreeMap<u64, super::spectral::Spectral> = BTreeMap::new();
        for _ in 0..read_u64(body, &mut at).map_err(|e| bad(&e))? {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let option_f32 = |body: &[u8], at: &mut usize| -> Result<Option<f32>, String> {
                let has = *body.get(*at).ok_or_else(|| String::from("a spectral record ends early"))?;
                *at += 1;
                let v = read_f32(body, at)?;
                match has {
                    1 if v.is_finite() => Ok(Some(v)),
                    1 => Err(String::from(
                        "a spectral record says it has a value and carries a non-finite one",
                    )),
                    0 => Ok(None),
                    _ => Err(String::from("a spectral presence byte is neither 0 nor 1")),
                }
            };
            let centroid_hz = option_f32(body, &mut at).map_err(|e| bad(&e))?;
            let rolloff_hz = option_f32(body, &mut at).map_err(|e| bad(&e))?;
            let noise_floor_db = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            spectral.insert(hash, super::spectral::Spectral { centroid_hz, rolloff_hz, noise_floor_db });
        }

        let mut tempo: BTreeMap<u64, super::tempo::Tempo> = BTreeMap::new();
        for _ in 0..read_u64(body, &mut at).map_err(|e| bad(&e))? {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let has_bpm = *body.get(at).ok_or_else(|| bad("a tempo record ends early"))?;
            at += 1;
            let bpm_value = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let bpm = match has_bpm {
                1 if bpm_value.is_finite() => Some(bpm_value),
                1 => return Err(bad("a tempo record says it has a bpm and carries a non-finite one")),
                0 => None,
                _ => return Err(bad("a tempo presence byte is neither 0 nor 1")),
            };
            let confidence = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let octave_ambiguous = match *body.get(at).ok_or_else(|| bad("a tempo record ends early"))? {
                1 => true,
                0 => false,
                _ => return Err(bad("a tempo octave byte is neither 0 nor 1")),
            };
            at += 1;
            tempo.insert(hash, super::tempo::Tempo { bpm, confidence, octave_ambiguous });
        }

        let mut key: BTreeMap<u64, super::key::Estimate> = BTreeMap::new();
        for _ in 0..read_u64(body, &mut at).map_err(|e| bad(&e))? {
            let hash = read_u64(body, &mut at).map_err(|e| bad(&e))?;
            let has_tonic = *body.get(at).ok_or_else(|| bad("a key record ends early"))?;
            at += 1;
            let tonic_value = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            let tonic = match has_tonic {
                1 if tonic_value.is_finite() => Some((tonic_value.round().max(0.0) as u8) % 12),
                1 => return Err(bad("a key record says it has a tonic and carries a non-finite one")),
                0 => None,
                _ => return Err(bad("a key presence byte is neither 0 nor 1")),
            };
            let major = match *body.get(at).ok_or_else(|| bad("a key record ends early"))? {
                1 => true,
                0 => false,
                _ => return Err(bad("a key mode byte is neither 0 nor 1")),
            };
            at += 1;
            let confidence = read_f32(body, &mut at).map_err(|e| bad(&e))?;
            key.insert(hash, super::key::Estimate { key: super::key::Key { tonic, major }, confidence });
        }

        if at != body.len() {
            return Err(bad(&format!("{} trailing bytes after the last section", body.len() - at)));
        }
        Ok(Index { stamp, records, provenance, loudness, spectral, tempo, key })
    }
}

/// A length-prefixed UTF-8 string, `u32` bytes long.
fn push_str(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

/// An optional string: a presence byte, then the length-prefixed value when present.
fn push_opt_str(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            push_str(out, v);
        }
    }
}

fn read_u32(bytes: &[u8], at: &mut usize) -> Result<u32, String> {
    let slice =
        bytes.get(*at..*at + 4).ok_or_else(|| format!("the file ends where a u32 was expected at {at}"))?;
    *at += 4;
    Ok(u32::from_le_bytes(slice.try_into().map_err(|_| String::from("a u32 is not 4 bytes"))?))
}

fn read_u64(bytes: &[u8], at: &mut usize) -> Result<u64, String> {
    let slice =
        bytes.get(*at..*at + 8).ok_or_else(|| format!("the file ends where a u64 was expected at {at}"))?;
    *at += 8;
    Ok(u64::from_le_bytes(slice.try_into().map_err(|_| String::from("a u64 is not 8 bytes"))?))
}

fn read_f32(bytes: &[u8], at: &mut usize) -> Result<f32, String> {
    let slice =
        bytes.get(*at..*at + 4).ok_or_else(|| format!("the file ends where an f32 was expected at {at}"))?;
    *at += 4;
    Ok(f32::from_le_bytes(slice.try_into().map_err(|_| String::from("an f32 is not 4 bytes"))?))
}

fn read_str(bytes: &[u8], at: &mut usize) -> Result<String, String> {
    let start = *at;
    let len = read_u32(bytes, at)? as usize;
    let slice = bytes.get(*at..*at + len).ok_or_else(|| format!("the file ends inside a string at {at}"))?;
    *at += len;
    String::from_utf8(slice.to_vec()).map_err(|_| format!("the string at {start} is not UTF-8"))
}

fn read_opt_str(bytes: &[u8], at: &mut usize) -> Result<Option<String>, String> {
    let has =
        *bytes.get(*at).ok_or_else(|| format!("the file ends where a presence byte was expected at {at}"))?;
    *at += 1;
    match has {
        0 => Ok(None),
        1 => read_str(bytes, at).map(Some),
        other => Err(format!("`{other}` is not a presence byte")),
    }
}
