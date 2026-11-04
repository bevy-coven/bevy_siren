//! **The golden set**: a labelled library and the text queries that ask it for things, loaded so that a mistake in either is
//! *named* rather than silently measured.
//!
//! `#35`'s point is that a change to a ranking is a guess until something fixed can say whether it helped. What is fixed is two
//! small files:
//!
//! - **labels** — `file,class`: which class each file of the library belongs to, as people labelled it. The dataset's own CSV
//!   (`filename,fold,target,category,…`) reads too, because the columns are named by the header, never by position.
//! - **queries** — tab-separated, header required: a `class`, a `description` and a `rationale` (other columns are ignored). The
//!   description is what a developer would type, the class says whose members are the right answers, and the rationale is one line
//!   on why. A class of `-` is a query that should find **nothing**.
//!
//! The loader's work is the refusals. A query with no rationale, one whose class labels no file, a description written twice
//! (rankings are keyed by description, so the second would silently replace the first), a labels file that names a file twice —
//! each is refused **naming the line**, because a golden set with a mistake in it measures the mistake. [`GoldenSet::absent`] and
//! [`GoldenSet::unlabelled`] are the same rule applied to a *library*: a labelled file that is not there, or a file that is there
//! and nobody labelled, would change every number, so a caller can refuse to run on either.
//!
//! This module holds no audio and judges nothing. The audio is a library on disk, the model is a caller's, and a person's judgement
//! of which neighbours *fit* an anchor is a different file (`#35`'s third criterion) that nothing here reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::library::Library;

use super::provenance::split_csv;

/// One text query of a golden set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenQuery {
    /// The class whose members are the right answers, or `None` for a query that should find nothing in this library.
    pub class: Option<String>,
    /// What a developer would type.
    pub description: String,
    /// One line: why the expected answer is right, or why nothing here should match.
    pub rationale: String,
}

/// Why a golden set could not be loaded. Every variant names the line or the class that is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoldenProblem {
    /// The labels could not be read as `file,class`.
    Labels {
        /// The 1-based line, or `0` for the file as a whole.
        line: usize,
        /// What is wrong.
        why: String,
    },
    /// The queries could not be read.
    Queries {
        /// The 1-based line, or `0` for the file as a whole.
        line: usize,
        /// What is wrong.
        why: String,
    },
    /// A query expects a class that labels no file, so its right answers would be empty and its miss would be blamed on the model.
    NoLabelledFile {
        /// The class the query names.
        class: String,
        /// The query.
        description: String,
    },
}

impl core::fmt::Display for GoldenProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let place = |line: &usize| if *line == 0 { String::new() } else { format!(", line {line}") };
        match self {
            GoldenProblem::Labels { line, why } => write!(f, "labels{}: {why}", place(line)),
            GoldenProblem::Queries { line, why } => write!(f, "queries{}: {why}", place(line)),
            GoldenProblem::NoLabelledFile { class, description } => write!(
                f,
                "`{description}` expects class `{class}` and no labelled file is in that class, so it would always miss and the miss \
                 would be blamed on the ranking"
            ),
        }
    }
}

// An error a caller can `?` into a `Box<dyn Error>` is a whole error; one it must unwrap by hand is half of one.
impl std::error::Error for GoldenProblem {}

/// **A golden set**: the labels of a library and the queries that ask it for things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenSet {
    /// Class by file, the file spelled as the labels spelled it: a path relative to the library root, or a bare file name.
    labels: BTreeMap<String, String>,
    /// The queries, in file order.
    pub queries: Vec<GoldenQuery>,
}

impl GoldenSet {
    /// **Load a set from the text of its two files**, refusing any mistake in either by name.
    ///
    /// Beyond what each file says on its own, a query that expects a class no file is labelled with is refused: it could never
    /// be answered, and the failure would read as the model's.
    pub fn parse(labels_csv: &str, queries_tsv: &str) -> Result<GoldenSet, GoldenProblem> {
        let labels = parse_labels(labels_csv)?;
        let queries = parse_queries(queries_tsv)?;
        let classes: BTreeSet<&str> = labels.values().map(String::as_str).collect();
        for query in &queries {
            if let Some(class) = &query.class
                && !classes.contains(class.as_str())
            {
                return Err(GoldenProblem::NoLabelledFile {
                    class: class.clone(),
                    description: query.description.clone(),
                });
            }
        }
        Ok(GoldenSet { labels, queries })
    }

    /// Every class that labels at least one file.
    pub fn classes(&self) -> BTreeSet<&str> {
        self.labels.values().map(String::as_str).collect()
    }

    /// How many files are labelled.
    pub fn labelled(&self) -> usize {
        self.labels.len()
    }

    /// **The class a library path is labelled with**: looked up as the labels spell paths (relative to the library root,
    /// `/`-separated), and failing that by bare file name, so a labels file that lists `1-100032-A-0.wav` and one that lists
    /// `audio/1-100032-A-0.wav` both work.
    pub fn label_of(&self, relative: &Path) -> Option<&str> {
        let path = relative.to_string_lossy().replace('\\', "/");
        self.labels
            .get(&path)
            .or_else(|| {
                relative.file_name().and_then(|name| self.labels.get(name.to_string_lossy().as_ref()))
            })
            .map(String::as_str)
    }

    /// **The right answers, by class**: every asset whose content is labelled with the class. An asset whose bytes appear at two
    /// paths with two classes is in both.
    pub fn right_answers(&self, library: &Library) -> BTreeMap<String, BTreeSet<u64>> {
        let mut by_class: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
        for asset in library.assets.values() {
            for path in &asset.paths {
                if let Some(class) = self.label_of(path) {
                    by_class.entry(class.to_string()).or_default().insert(asset.hash);
                }
            }
        }
        by_class
    }

    /// **Labelled files the library does not have.** A partial copy of a set measures a different set, so a caller refuses to
    /// run on one — which is the one thing a number compared against a recorded baseline cannot survive.
    pub fn absent(&self, library: &Library) -> Vec<String> {
        let (mut paths, mut names) = (BTreeSet::new(), BTreeSet::new());
        for path in library.assets.values().flat_map(|asset| asset.paths.iter()) {
            paths.insert(path.to_string_lossy().replace('\\', "/"));
            if let Some(name) = path.file_name() {
                names.insert(name.to_string_lossy().into_owned());
            }
        }
        self.labels.keys().filter(|file| !paths.contains(*file) && !names.contains(*file)).cloned().collect()
    }

    /// **Files in the library that nobody labelled**, in path order. One that slipped in would join every ranking as a wrong
    /// answer and change every number, so a caller refuses to run on a library that has any.
    pub fn unlabelled(&self, library: &Library) -> Vec<PathBuf> {
        let mut stray: Vec<PathBuf> = library
            .assets
            .values()
            .flat_map(|asset| asset.paths.iter())
            .filter(|path| self.label_of(path).is_none())
            .cloned()
            .collect();
        stray.sort();
        stray
    }
}

/// `file,class` by header name; the dataset's own `filename,…,category` spelling reads too.
fn parse_labels(text: &str) -> Result<BTreeMap<String, String>, GoldenProblem> {
    let problem = |line: usize, why: String| GoldenProblem::Labels { line, why };
    let mut rows =
        text.lines().enumerate().map(|(at, line)| (at + 1, line)).filter(|(_, line)| !line.trim().is_empty());
    let Some((_, header)) = rows.next() else {
        return Err(problem(0, String::from("the file is empty, so no file has a class")));
    };
    let columns: Vec<String> = split_csv(header).iter().map(|c| c.trim().to_ascii_lowercase()).collect();
    let find = |names: &[&str]| columns.iter().position(|c| names.contains(&c.as_str()));
    let (Some(file_at), Some(class_at)) =
        (find(&["file", "filename", "path"]), find(&["class", "category", "label"]))
    else {
        return Err(problem(
            1,
            String::from(
                "the header names no `file` column and `class` column (`filename` and `category` also read), so no row can say \
                 which class a file is in",
            ),
        ));
    };
    let mut labels = BTreeMap::new();
    for (line, row) in rows {
        let cells = split_csv(row);
        let cell = |at: usize| cells.get(at).map(|c| c.trim()).unwrap_or("");
        let (file, class) = (cell(file_at).replace('\\', "/"), cell(class_at));
        if file.is_empty() || class.is_empty() {
            return Err(problem(line, String::from("a row with no file or no class")));
        }
        if labels.insert(file.clone(), class.to_string()).is_some() {
            return Err(problem(
                line,
                format!("`{file}` is labelled twice, so which class it is in is not answerable"),
            ));
        }
    }
    Ok(labels)
}

/// Tab-separated, `#` comments and blank lines skipped, the first remaining line the header.
fn parse_queries(text: &str) -> Result<Vec<GoldenQuery>, GoldenProblem> {
    let problem = |line: usize, why: String| GoldenProblem::Queries { line, why };
    let mut rows = text
        .lines()
        .enumerate()
        .map(|(at, line)| (at + 1, line))
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
    let Some((header_line, header)) = rows.next() else {
        return Err(problem(0, String::from("the file has no header, so no column is named")));
    };
    let columns: Vec<String> = header.split('\t').map(|c| c.trim().to_ascii_lowercase()).collect();
    let find = |name: &str| columns.iter().position(|c| c == name);
    let (Some(class_at), Some(description_at), Some(rationale_at)) =
        (find("class"), find("description"), find("rationale"))
    else {
        return Err(problem(
            header_line,
            String::from(
                "the header must name `class`, `description` and `rationale` columns, separated by tabs",
            ),
        ));
    };

    let mut queries = Vec::new();
    let mut first_seen: BTreeMap<String, usize> = BTreeMap::new();
    for (line, row) in rows {
        let cells: Vec<&str> = row.split('\t').collect();
        let cell = |at: usize| cells.get(at).map(|c| c.trim()).unwrap_or("");
        let (class, description, rationale) = (cell(class_at), cell(description_at), cell(rationale_at));
        if description.is_empty() {
            return Err(problem(line, String::from("a query with no description")));
        }
        if class.is_empty() {
            return Err(problem(
                line,
                format!("`{description}` names no class: write `-` for a query that should find nothing"),
            ));
        }
        if rationale.is_empty() {
            return Err(problem(
                line,
                format!(
                    "`{description}` has no rationale: every query says why its answer is right, or why nothing should match"
                ),
            ));
        }
        if let Some(first) = first_seen.insert(description.to_lowercase(), line) {
            return Err(problem(
                line,
                format!(
                    "`{description}` repeats line {first}: rankings are keyed by description, so the second would silently \
                     replace the first"
                ),
            ));
        }
        queries.push(GoldenQuery {
            class: (class != "-").then(|| class.to_string()),
            description: description.to_string(),
            rationale: rationale.to_string(),
        });
    }
    Ok(queries)
}
