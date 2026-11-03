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

use super::provenance::Provenance;
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
        Index { stamp, records: BTreeMap::new(), provenance: BTreeMap::new() }
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
        let mut expected_provenance: Option<usize> = None;
        // **The section marker is the only line that can start with `provenance `**, because every other line begins
        // with a 16-hex-character hash. Everything after it is a provenance record, not a vector.
        let mut in_provenance = false;
        for (n, line) in body.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let line_no = n + 8;
            if in_provenance {
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
            if let Some(count) = line.strip_prefix("provenance ") {
                expected_provenance = Some(count.parse().map_err(|_| IndexProblem::Record {
                    line: line_no,
                    what: format!("`{count}` is not a provenance record count"),
                })?);
                in_provenance = true;
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

        Ok(Index { stamp, records, provenance })
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`IndexProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for IndexProblem {}
