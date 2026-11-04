//! **Where a sound came from, and what its terms are.** The licence obligation that has to travel with a file.
//!
//! A game built from a purchased or downloaded asset library carries a licensing obligation **per file**, and a tool that
//! mixes assets into a project should carry that obligation forward rather than lose it. Once an index is built without
//! provenance the only recovery is a rescan of files that may no longer exist — which is why this is cheap now and
//! impossible later.
//!
//! # The one rule: unknown is a value, not an absence
//!
//! Every field here is an [`Option`] or a [`Source::Unknown`], and the distinction between *"nobody recorded this"* and
//! *"somebody recorded that it is empty"* is load-bearing:
//!
//! | Recorded | Means |
//! |---|---|
//! | `licence: None` | no licence was recorded — **treat as not-permissive** |
//! | `licence: Some("")` | a sidecar deliberately wrote an empty string — also not a permission, but a *recording* |
//!
//! [`Provenance::permits_commercial_use`] is the only place that turns either into an answer, and it answers `false` for
//! both. **Unknown provenance is never presented as permissive**, which is the issue's fourth criterion and the reason
//! the answer is not a `bool` field someone could default to `true`.
//!
//! # The pitfall this refuses to fall into
//!
//! A library's `README` frequently says the whole folder is CC0. That is **not** a per-asset record, and inheriting it
//! would produce a licence that *looks authoritative* and is not — worse than an honest unknown, because a person
//! checking the file would stop. So nothing here reads a folder-level statement; a `LICENSE` or `README` beside the
//! assets is deliberately ignored, and the only inputs are a per-file record.

use std::collections::BTreeMap;
use std::path::Path;

/// **What is known about where an asset came from.**
///
/// Every field is optional and defaults to "not recorded", because a library assembled from several sources — a Kenney
/// pack, a Freesound download, a purchased set — has different answers for different files and no answer at all for
/// some. A missing field is `None`, never an empty string standing in for one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Provenance {
    /// The licence as the sidecar spelled it, e.g. `CC0-1.0`, `CC-BY-4.0`, `MIT`.
    ///
    /// Stored as **the string that was recorded**, not parsed into an enum: a licence identifier this crate does not
    /// know is still a licence a person can read, and refusing it would lose the recording. [`Provenance::permits_
    /// commercial_use`] is where the strings that *are* understood are interpreted.
    pub licence: Option<String>,
    /// Where the file came from: a URL, a pack name, a purchase order.
    pub source: Option<Source>,
    /// Who to credit, when the licence requires attribution.
    pub author: Option<String>,
}

/// **Where a file came from**, when the sidecar said.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Source {
    /// A URL the asset was downloaded from.
    Url(String),
    /// A named pack or library, e.g. `Kenney Impact Sounds`.
    Pack(String),
    /// **Recorded as unknown.** Different from [`Provenance::source`] being `None`: this says a record *exists* and it
    /// says the origin is not known, which is what a person needs to see.
    Unknown,
}

impl Provenance {
    /// Nothing is known: the state of every asset in a library with no sidecar at all.
    pub fn unknown() -> Self {
        Provenance::default()
    }

    /// Whether **anything** was recorded, so a report can say *"204 of 1,000 assets have provenance"*.
    pub fn is_recorded(&self) -> bool {
        self.licence.is_some() || self.source.is_some() || self.author.is_some()
    }

    /// **Whether the recorded terms permit commercial use**, or `None` when this crate cannot say.
    ///
    /// Three outcomes, and the third is why this is not a `bool`:
    ///
    /// | Answer | Meaning |
    /// |---|---|
    /// | `Some(true)` | a licence this crate recognises as commercial-friendly, e.g. `CC0-1.0`, `MIT`, `Apache-2.0` |
    /// | `Some(false)` | a licence it recognises as **not** commercial, e.g. `CC-BY-NC-4.0` |
    /// | `None` | nothing recorded, or a licence string it does not recognise — **a person must decide** |
    ///
    /// A caller that treats `None` as `Some(true)` has reintroduced the bug this type exists to prevent. `#14`'s
    /// criterion is that unknown is *never presented as permissive*, so the defaulting is left to the caller's code
    /// rather than hidden here.
    pub fn permits_commercial_use(&self) -> Option<bool> {
        let licence = self.licence.as_deref()?.trim();
        if licence.is_empty() {
            return None;
        }
        // Matched case-insensitively on the identifier's stem, because a sidecar may write `cc0`, `CC0-1.0` or
        // `CC0 1.0`. The list is deliberately short: an identifier this crate does not know returns `None` rather than
        // being assumed permissive.
        let lower = licence.to_ascii_lowercase();
        // **Non-commercial is checked first and by substring**, because the token that marks it (`-nc`) can sit
        // anywhere in an identifier and a missed one is the dangerous direction: it would report a non-commercial
        // licence as usable in a game.
        let non_commercial =
            lower.contains("-nc") || lower.starts_with("nc-") || lower.contains("noncommercial");
        if non_commercial {
            return Some(false);
        }
        // **Permissive is matched as a leading stem, never a substring.** `contains("mit")` would accept
        // `"resubmitted"` — and this function's whole job is to refuse to over-claim, so the looser match is exactly
        // the bug it exists to prevent.
        const PERMISSIVE_STEMS: &[&str] =
            &["cc0", "cc-by", "public domain", "mit", "apache", "bsd", "isc", "unlicense", "mpl-2"];
        if PERMISSIVE_STEMS.iter().any(|known| lower.starts_with(known)) { Some(true) } else { None }
    }
}

impl core::fmt::Display for Provenance {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let licence = self.licence.as_deref().unwrap_or("unknown");
        match &self.source {
            Some(Source::Url(u)) => write!(f, "{licence}, from {u}"),
            Some(Source::Pack(p)) => write!(f, "{licence}, from `{p}`"),
            Some(Source::Unknown) => write!(f, "{licence}, source unknown"),
            None => write!(f, "{licence}, source not recorded"),
        }
    }
}

/// Why a sidecar could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceProblem {
    /// The sidecar names two records for one file, so which one is authoritative is not answerable here.
    Conflicting {
        /// The file both records claim.
        file: String,
    },
    /// The sidecar exists and would not parse, carrying the parser's own words.
    Unreadable {
        /// The sidecar's path.
        path: String,
        /// What the parser said.
        why: String,
    },
}

impl core::fmt::Display for ProvenanceProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProvenanceProblem::Conflicting { file } => write!(
                f,
                "two records name `{file}` and this crate will not choose between them: a licence that is a guess is \
                 worse than one that is unknown"
            ),
            ProvenanceProblem::Unreadable { path, why } => {
                write!(f, "`{path}` could not be read as a licence record: {why}")
            }
        }
    }
}

impl std::error::Error for ProvenanceProblem {}

/// **What a sidecar said, keyed by the file it describes.**
///
/// Keyed by **file name relative to the library root**, not by content hash, because a sidecar is written by whoever
/// published the pack and it names files. The link to a hash is made by the caller that has both.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sidecar {
    /// One record per file, keyed by its relative path as the sidecar spelled it.
    pub records: BTreeMap<String, Provenance>,
}

/// The sidecar file names this crate looks for, in order. The first that exists is used; more than one is a conflict
/// the caller resolves rather than a merge this crate invents.
pub const SIDECAR_NAMES: &[&str] = &["licences.csv", "licenses.csv", "licences.json", "licenses.json"];

impl Sidecar {
    /// **Read one sidecar file**, choosing the format from its extension.
    ///
    /// CSV is the shape a purchased pack ships (one row per file, a header naming the columns) and JSON is what a
    /// generated library has. Both map to the same [`Provenance`].
    pub fn read(path: &Path) -> Result<Sidecar, ProvenanceProblem> {
        let text = std::fs::read_to_string(path).map_err(|e| ProvenanceProblem::Unreadable {
            path: path.display().to_string(),
            why: e.to_string(),
        })?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        if extension == "json" { Self::from_json(&text, path) } else { Self::from_csv(&text, path) }
    }

    /// **CSV, with the columns named by the header** rather than by position, so a pack that orders its columns
    /// differently still reads.
    ///
    /// Recognised headers, case-insensitive: `file`/`path`/`filename`, `licence`/`license`, `source`,
    /// `author`/`creator`/`artist`. A file whose row has no recognised `file` column is a row this crate cannot attach
    /// to an asset, and it is skipped rather than guessed at.
    pub fn from_csv(text: &str, path: &Path) -> Result<Sidecar, ProvenanceProblem> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let header = match lines.next() {
            Some(h) => h,
            None => return Ok(Sidecar::default()),
        };
        let columns: Vec<String> = split_csv(header).iter().map(|c| c.trim().to_ascii_lowercase()).collect();
        let find = |names: &[&str]| columns.iter().position(|c| names.contains(&c.as_str()));
        let Some(file_at) = find(&["file", "path", "filename"]) else {
            // No column names a file, so no row can be attached to an asset. That is a sidecar this crate cannot use,
            // and saying so beats returning an empty map that looks like "a library with no licences".
            return Err(ProvenanceProblem::Unreadable {
                path: path.display().to_string(),
                why: String::from(
                    "the header names no `file`, `path` or `filename` column, so no row can be attached to an asset",
                ),
            });
        };
        let licence_at = find(&["licence", "license"]);
        let source_at = find(&["source", "url", "origin"]);
        let author_at = find(&["author", "creator", "artist"]);

        let mut sidecar = Sidecar::default();
        for line in lines {
            let row = split_csv(line);
            let Some(file) = row.get(file_at).map(|s| s.trim()).filter(|s| !s.is_empty()) else { continue };
            let field = |at: Option<usize>| {
                at.and_then(|i| row.get(i))
                    .map(|s| s.trim().to_string())
                    // An empty cell and a missing cell are both "not recorded", which is the distinction this type
                    // deliberately does not collapse into an empty string.
                    .filter(|s| !s.is_empty())
            };
            let provenance = Provenance {
                licence: field(licence_at),
                source: field(source_at)
                    .map(|s| if s.starts_with("http") { Source::Url(s) } else { Source::Pack(s) }),
                author: field(author_at),
            };
            if sidecar.records.insert(file.to_string(), provenance).is_some() {
                return Err(ProvenanceProblem::Conflicting { file: file.to_string() });
            }
        }
        Ok(sidecar)
    }

    /// **JSON**, as either `{"file": {...}}` or `[{"file": "...", ...}]`.
    ///
    /// Hand-parsed rather than pulling a JSON dependency: the crate's list is closed (`tests/leaf.rs`) and this format
    /// is flat. It understands objects, strings, and `null`, which is all a licence record needs.
    pub fn from_json(text: &str, path: &Path) -> Result<Sidecar, ProvenanceProblem> {
        let unreadable = |why: &str| ProvenanceProblem::Unreadable {
            path: path.display().to_string(),
            why: why.to_string(),
        };
        // The array form is the one a generated library uses; the object form is what a person writes by hand.
        let trimmed = text.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return Err(unreadable("a licence record must be a JSON object or array"));
        }
        // Split on top-level entries. A full parser is out of scope here; a record's fields are flat strings, so the
        // scan is over balanced braces and quoted strings — which is enough for every sidecar this crate documents and
        // is stated as a limit rather than presented as a general JSON reader.
        let mut sidecar = Sidecar::default();
        for entry in top_level_entries(trimmed) {
            let (Some(file), licence, source, author) = parse_record(&entry) else { continue };
            let provenance = Provenance {
                licence,
                source: source.map(|s| if s.starts_with("http") { Source::Url(s) } else { Source::Pack(s) }),
                author,
            };
            if sidecar.records.insert(file.clone(), provenance).is_some() {
                return Err(ProvenanceProblem::Conflicting { file });
            }
        }
        Ok(sidecar)
    }
}

/// Split one CSV line on commas, respecting double quotes. Not a general CSV reader: it handles the quoted-comma case a
/// licence field like `"CC-BY-4.0, attribution required"` produces, and nothing beyond it.
pub(crate) fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(std::mem::take(&mut current));
            }
            c => current.push(c),
        }
    }
    out.push(current);
    out
}

/// The `{...}` or `"key": {...}` entries at the top level of a JSON document, as raw text.
fn top_level_entries(text: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut start: Option<usize> = None;
    for (i, ch) in text.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '{' if !quoted => {
                depth += 1;
                if depth == 1 {
                    start = Some(i);
                }
            }
            '}' if !quoted => {
                if depth == 1
                    && let Some(s) = start.take()
                {
                    entries.push(text[s..=i].to_string());
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    entries
}

/// One JSON record's `file`, `licence`, `source` and `author`, when the record names a file.
fn parse_record(entry: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    (
        json_field(entry, &["file", "path"]),
        json_field(entry, &["licence", "license"]),
        json_field(entry, &["source"]),
        json_field(entry, &["author", "creator"]),
    )
}

/// A flat string field of a JSON object, matched case-insensitively, or `None` for a missing, `null` or empty value.
fn json_field(entry: &str, names: &[&str]) -> Option<String> {
    let lower = entry.to_ascii_lowercase();
    let mut from = 0usize;
    let mut best: Option<(usize, String)> = None;
    for name in names {
        let needle = format!("\"{name}\"");
        if let Some(found) = lower[from.min(lower.len())..].find(&needle) {
            let at = from + found;
            let after = &entry[at + needle.len()..];
            let value = after.trim_start().strip_prefix(':')?.trim_start();
            let value = value.strip_prefix('"')?;
            let end = value.find('"')?;
            let text = value[..end].to_string();
            if !text.is_empty() && best.as_ref().is_none_or(|(b, _)| at < *b) {
                best = Some((at, text));
                from = at;
            }
        }
    }
    best.map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_recorded_is_an_unknown_not_an_empty_licence() {
        let p = Provenance::unknown();
        assert!(!p.is_recorded());
        assert_eq!(p.permits_commercial_use(), None, "unknown is not a permission");
        assert_eq!(p.to_string(), "unknown, source not recorded");
    }

    /// **The issue's first criterion**: an asset with no metadata is distinguishable from one recorded as empty.
    #[test]
    fn an_absent_licence_is_distinguishable_from_an_empty_one() {
        let absent = Provenance::unknown();
        let empty = Provenance { licence: Some(String::new()), ..Provenance::unknown() };
        assert!(!absent.is_recorded(), "absent records nothing");
        assert!(empty.is_recorded(), "an empty string IS a recording, so it is recorded");
        // Both refuse to claim permission, which is the part that must not differ.
        assert_eq!(absent.permits_commercial_use(), None);
        assert_eq!(empty.permits_commercial_use(), None);
    }

    /// **The issue's fourth criterion**: unknown is never presented as permissive.
    #[test]
    fn unknown_provenance_never_reads_as_permissive() {
        assert_eq!(Provenance::unknown().permits_commercial_use(), None);
        assert_eq!(
            Provenance { licence: Some(String::from("SomeCustomEULA")), ..Provenance::unknown() }
                .permits_commercial_use(),
            None,
            "an identifier this crate does not know is a question for a person, not a yes"
        );
    }

    #[test]
    fn the_licences_it_does_know_answer_correctly() {
        let with = |l: &str| Provenance { licence: Some(l.to_string()), ..Provenance::unknown() };
        assert_eq!(with("CC0-1.0").permits_commercial_use(), Some(true));
        assert_eq!(with("cc0 1.0").permits_commercial_use(), Some(true), "case and separator insensitive");
        assert_eq!(with("MIT").permits_commercial_use(), Some(true));
        assert_eq!(with("Apache-2.0").permits_commercial_use(), Some(true));
        assert_eq!(with("CC-BY-NC-4.0").permits_commercial_use(), Some(false));
        assert_eq!(with("CC-BY-NC 4.0").permits_commercial_use(), Some(false));
        assert_eq!(with("NonCommercial").permits_commercial_use(), Some(false));
        // `BY` alone is commercial, and must not be caught by the `-nc` check.
        assert_eq!(with("CC-BY-4.0").permits_commercial_use(), Some(true));
        assert_eq!(
            with("CC-BY-SA-4.0").permits_commercial_use(),
            Some(true),
            "share-alike is a separate obligation"
        );
        assert_eq!(
            with("CC-BY-ND-4.0").permits_commercial_use(),
            Some(true),
            "no-derivatives is a separate obligation"
        );

        // **The over-claim this refuses.** A substring match would accept anything containing "mit".
        assert_eq!(
            with("resubmitted-terms").permits_commercial_use(),
            None,
            "a stem match, not a substring: `resubmitted` contains `mit` and is not the MIT licence"
        );
        assert_eq!(with("Proprietary").permits_commercial_use(), None);
    }

    #[test]
    fn a_sidecar_names_columns_rather_than_positions() {
        let csv = "source,file,licence\nkenney,step.wav,CC0-1.0\n";
        let s = Sidecar::from_csv(csv, Path::new("licences.csv")).expect("read");
        let step = &s.records["step.wav"];
        assert_eq!(step.licence.as_deref(), Some("CC0-1.0"));
        assert_eq!(step.source, Some(Source::Pack(String::from("kenney"))));
    }

    #[test]
    fn a_quoted_comma_inside_a_licence_survives() {
        let csv = "file,licence\nstep.wav,\"CC-BY-4.0, attribution required\"\n";
        let s = Sidecar::from_csv(csv, Path::new("l.csv")).expect("read");
        assert_eq!(s.records["step.wav"].licence.as_deref(), Some("CC-BY-4.0, attribution required"));
    }

    #[test]
    fn a_sidecar_with_no_file_column_is_refused_rather_than_read_as_empty() {
        let csv = "licence,source\nCC0-1.0,kenney\n";
        let err = Sidecar::from_csv(csv, Path::new("l.csv")).expect_err("no file column");
        match err {
            ProvenanceProblem::Unreadable { why, .. } => assert!(why.contains("no `file`")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn two_rows_for_one_file_are_a_conflict_not_a_last_write_wins() {
        let csv = "file,licence\na.wav,CC0-1.0\na.wav,CC-BY-NC-4.0\n";
        let err = Sidecar::from_csv(csv, Path::new("l.csv")).expect_err("conflict");
        assert_eq!(err, ProvenanceProblem::Conflicting { file: String::from("a.wav") });
    }

    #[test]
    fn an_empty_cell_records_nothing() {
        let csv = "file,licence,author\na.wav,,\n";
        let s = Sidecar::from_csv(csv, Path::new("l.csv")).expect("read");
        let a = &s.records["a.wav"];
        assert_eq!(a.licence, None);
        assert_eq!(a.author, None);
        assert!(!a.is_recorded(), "a row of empty cells records nothing");
    }

    #[test]
    fn json_records_read_by_name() {
        let json =
            r#"[{"file":"a.wav","licence":"CC0-1.0","source":"https://x"},{"file":"b.wav","license":"MIT"}]"#;
        let s = Sidecar::from_json(json, Path::new("l.json")).expect("read");
        assert_eq!(s.records["a.wav"].licence.as_deref(), Some("CC0-1.0"));
        assert_eq!(s.records["a.wav"].source, Some(Source::Url(String::from("https://x"))));
        assert_eq!(
            s.records["b.wav"].licence.as_deref(),
            Some("MIT"),
            "`license` is accepted beside `licence`"
        );
    }

    #[test]
    fn an_unreadable_json_document_is_named() {
        let err = Sidecar::from_json("not json", Path::new("l.json")).expect_err("refused");
        matches!(err, ProvenanceProblem::Unreadable { .. });
    }
}
