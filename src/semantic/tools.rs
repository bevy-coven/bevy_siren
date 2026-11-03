//! **The workflow as calls**: anchor, compatible music, compatible effects, and *why*.
//!
//! The retrieval engine answers *"what is near this description"*. A person's workflow asks a different question:
//! *"I picked this one; what belongs beside it, and why?"* That is `#42`, `#43` and `#44`, and they are library calls
//! rather than an MCP server because the editor's in-process panel needs the same functions — a tool surface that only
//! an agent can reach would be a second implementation of the same rules.
//!
//! # Why the tools are functions and the session is a type
//!
//! `set_anchor` makes this stateful, and `#42` asks for that scope to be stated rather than implied. [`Session`] holds
//! **one anchor and nothing else**, and it is the caller's to create. There is no global: two clients each hold their
//! own, so the interleaving `#42` warns about cannot happen, and a restart loses the anchor because a restart should.
//!
//! # Music and effects are different weightings, not different code
//!
//! An effect has no key and no tempo — those are `None`, never zero — so the two signal weights that would read them
//! are dead weight in a music table. [`Toolset::effects`] therefore carries its own [`Weights`], and the two defaults
//! are separate documented values rather than one number changed quietly. `#43`'s own words.
//!
//! # Near-identical variants
//!
//! `#43`: *"an effect library often has hundreds of near-identical variants (footstep 01–12). The result must not return
//! twelve variants of one footstep as the top twelve."* [`Toolset::compatible_music`] and [`Toolset::compatible_sfx`] enforce [`Variants`]: a result whose
//! embedding is within `identical_below` cosine of one already returned is **reported** as an omitted variant rather
//! than silently dropped, because a caller choosing between three footsteps still wants to know there are nine more.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Breakdown;

use super::filter::Features;
use super::index::{Index, ModelStamp};
use super::rank::{Floor, cosine};
use super::weights::Weights;

/// **How near two embeddings must be to count as variants of one sound**, and how many to keep.
///
/// **Design default `identical_below: 0.99`.** Two assets at a cosine above 0.99 are, as far as the encoder can tell,
/// the same sound; below it they are a different take. The number is high on purpose: `#43`'s pitfall is a *list* of
/// twelve near-duplicates, not a list of two sounds that resemble each other, and a lower threshold would suppress
/// genuinely different takes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Variants {
    /// Two results at or above this cosine are treated as the same sound.
    pub identical_below: f32,
}

impl Default for Variants {
    fn default() -> Self {
        Variants { identical_below: 0.99 }
    }
}

/// **What a tool could not do**, each naming what to do about it.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolProblem {
    /// No anchor was given and none was set.
    NoAnchor,
    /// The anchor's hash is not in the index, carrying the hash so the message points at it.
    AnchorNotIndexed {
        /// The hash.
        anchor: u64,
    },
    /// The search itself refused.
    Search(String),
}

impl core::fmt::Display for ToolProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ToolProblem::NoAnchor => {
                f.write_str("no anchor is set. Call `set_anchor(asset_id)` first, or pass one explicitly")
            }
            ToolProblem::AnchorNotIndexed { anchor } => {
                write!(f, "the anchor {anchor:016x} is not in this index: build it, or index the file")
            }
            ToolProblem::Search(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ToolProblem {}

/// **One anchor, and what the tools know about it.**
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// The asset's content hash.
    pub hash: u64,
    /// Its embedding, so a neighbour search needs no second lookup.
    pub embedding: Vec<f32>,
    /// Its acoustic features, so every candidate can be judged against it.
    pub features: Features,
}

/// **One result, with its reasons** — the shape every tool in this module returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The asset's content hash.
    pub hash: u64,
    /// The ensemble's score.
    pub score: f32,
    /// The embedding similarity, on the model's own scale.
    pub similarity: f32,
    /// Every signal's raw value, weight and contribution.
    pub breakdown: Breakdown,
    /// Where the asset came from.
    pub provenance: super::provenance::Provenance,
}

impl Found {
    /// **The sentence a panel or an agent shows**: what carried it, what counted against it, what was never measured.
    pub fn explain(&self) -> String {
        self.breakdown.explain()
    }
}

/// **What one answer left out**, so an empty or short list is a fact rather than a mystery.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tally {
    /// Results returned.
    pub returned: usize,
    /// Candidates excluded by the hard acoustic rules.
    pub excluded: usize,
    /// Candidates below the similarity floor.
    pub below_floor: usize,
    /// Candidates that were variants of one already returned, with the hash of the one they resembled.
    pub variants: Vec<(u64, u64)>,
    /// Candidates removed by the caller's category filter.
    pub wrong_category: usize,
}

impl Tally {
    /// A sentence naming every count, in the order they were applied.
    pub fn describe(&self) -> String {
        let mut parts = vec![format!("{} returned", self.returned)];
        if self.excluded > 0 {
            parts.push(format!("{} excluded by the acoustic rules", self.excluded));
        }
        if self.below_floor > 0 {
            parts.push(format!("{} below the similarity floor", self.below_floor));
        }
        if !self.variants.is_empty() {
            parts.push(format!("{} near-identical variants", self.variants.len()));
        }
        if self.wrong_category > 0 {
            parts.push(format!("{} outside the category", self.wrong_category));
        }
        parts.join(", ")
    }
}

/// **The call's answer**: the results and what was left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Answer {
    /// The results, best first by ensemble score.
    pub results: Vec<Found>,
    /// What was left out, and why.
    pub tally: Tally,
    /// The weights this answer used, so a caller can reproduce it.
    pub weights: Weights,
}

impl Answer {
    /// A sentence a caller can show without composing one.
    pub fn describe(&self) -> String {
        if self.results.is_empty() {
            return format!("nothing fit this anchor; {}", self.tally.describe());
        }
        self.tally.describe()
    }
}

/// **A tool session: one anchor, held by the caller.**
///
/// `#42`'s pitfall is two clients setting different anchors on one server. There is no shared state here to interleave:
/// each `Session` holds its own anchor, so the scope is *one session object*, lost when it drops.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Session {
    anchor: Option<u64>,
}

impl Session {
    /// A session with no anchor.
    pub fn new() -> Session {
        Session { anchor: None }
    }

    /// **Set the anchor**, returning it and what it knows about itself.
    ///
    /// Setting the same anchor twice is idempotent by construction: the field holds a hash, so the second call stores
    /// the same value and every later call sees the same context.
    pub fn set_anchor(&mut self, index: &Index, hash: u64) -> Result<Anchor, ToolProblem> {
        let embedding =
            index.records.get(&hash).cloned().ok_or(ToolProblem::AnchorNotIndexed { anchor: hash })?;
        self.anchor = Some(hash);
        Ok(Anchor { hash, embedding, features: features_of(index, hash) })
    }

    /// The anchor currently set, if any.
    pub fn anchor(&self) -> Option<u64> {
        self.anchor
    }

    /// Forget the anchor, so a later call must name one.
    pub fn clear_anchor(&mut self) {
        self.anchor = None;
    }

    /// **The anchor a call should use**: the one passed, or else the session's.
    ///
    /// An explicit argument wins, because a caller passing one is asking a question about *that* asset. With neither,
    /// the refusal names how to set one — `#42`'s third criterion.
    fn resolve_anchor(&self, given: Option<u64>) -> Result<u64, ToolProblem> {
        given.or(self.anchor).ok_or(ToolProblem::NoAnchor)
    }
}

/// **Build a [`Features`] from the index's own maps**, so a caller does not assemble one by hand and miss a section.
pub fn features_of(index: &Index, hash: u64) -> Features {
    Features {
        loudness: index.loudness.get(&hash).copied(),
        spectral: index.spectral.get(&hash).copied(),
        tempo: index.tempo.get(&hash).copied(),
        key: index.key.get(&hash).copied(),
    }
}

/// **The two weightings.** Music reads key and tempo; effects cannot, so it spends their weight on acoustic agreement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toolset {
    /// The weights a music query uses.
    pub music: Weights,
    /// The weights an effect query uses.
    pub effects: Weights,
    /// How near two results must be to count as one sound.
    pub variants: Variants,
}

impl Default for Toolset {
    fn default() -> Self {
        Toolset { music: Weights::default(), effects: Weights::effects(), variants: Variants::default() }
    }
}

impl Toolset {
    /// **Effects that fit `anchor`'s world.**
    ///
    /// `category` is a caller-supplied string matched against an asset's provenance `source`, because `#43` puts an
    /// authored taxonomy out of scope: a category is whatever the caller's file names say.
    pub fn compatible_sfx(
        &self,
        index: &Index,
        session: &Session,
        anchor: Option<u64>,
        category: Option<&str>,
        k: usize,
    ) -> Result<Answer, ToolProblem> {
        self.compatible(index, session, anchor, category, k, self.effects)
    }

    /// **Music that fits `anchor`**, using the music weighting.
    pub fn compatible_music(
        &self,
        index: &Index,
        session: &Session,
        anchor: Option<u64>,
        k: usize,
    ) -> Result<Answer, ToolProblem> {
        self.compatible(index, session, anchor, None, k, self.music)
    }

    /// **The shared body**: rank by cosine, apply the hard rules, weight, then suppress near-identical variants.
    fn compatible(
        &self,
        index: &Index,
        session: &Session,
        anchor: Option<u64>,
        category: Option<&str>,
        k: usize,
        weights: Weights,
    ) -> Result<Answer, ToolProblem> {
        let anchor_hash = session.resolve_anchor(anchor)?;
        let anchor_embedding =
            index.records.get(&anchor_hash).ok_or(ToolProblem::AnchorNotIndexed { anchor: anchor_hash })?;
        let anchor_features = features_of(index, anchor_hash);

        let mut tally = Tally::default();
        let mut scored: Vec<Found> = Vec::new();
        for (hash, vector) in &index.records {
            if *hash == anchor_hash {
                continue; // The anchor is not its own neighbour.
            }
            if let Some(want) = category {
                let source = index.provenance.get(hash).and_then(|p| p.source.as_ref());
                if !source_of(source).contains(want) {
                    tally.wrong_category += 1;
                    continue;
                }
            }
            let Some(similarity) = cosine(anchor_embedding, vector) else {
                continue;
            };
            if similarity < Floor::default().min_score {
                tally.below_floor += 1;
                continue;
            }
            let features = features_of(index, *hash);
            let judgement = super::filter::evaluate(&anchor_features, &features, &weights);
            if !judgement.admitted() {
                tally.excluded += 1;
                continue;
            }
            let breakdown =
                super::ensemble::breakdown(similarity, &features, &anchor_features, &judgement, &weights);
            scored.push(Found {
                hash: *hash,
                score: breakdown.score(),
                similarity,
                breakdown,
                provenance: index.provenance.get(hash).cloned().unwrap_or_default(),
            });
        }

        // **Descending score, then ascending hash**: a total order, so two runs agree field for field.
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| a.hash.cmp(&b.hash))
        });

        // **Variants, after ordering**, so the keeper is the best-scoring one rather than an arbitrary file.
        let mut kept: Vec<Found> = Vec::with_capacity(scored.len());
        'candidate: for candidate in scored {
            let Some(vector) = index.records.get(&candidate.hash) else { continue };
            for already in &kept {
                let Some(other) = index.records.get(&already.hash) else { continue };
                if cosine(vector, other).is_some_and(|c| c >= self.variants.identical_below) {
                    tally.variants.push((candidate.hash, already.hash));
                    continue 'candidate;
                }
            }
            kept.push(candidate);
        }

        kept.truncate(k);
        tally.returned = kept.len();
        Ok(Answer { results: kept, tally, weights })
    }

    /// **Why `candidate` scored what it did against `anchor`** — `#44`, and the tool that makes a bad ranking diagnosable.
    ///
    /// The breakdown is built by the same [`super::ensemble::breakdown`] a search uses, so its numbers **are** the
    /// numbers `#40` produced for that pair; a test asserts that rather than assuming it.
    pub fn explain_match(&self, index: &Index, anchor: u64, candidate: u64) -> Result<Found, ToolProblem> {
        let anchor_embedding = index.records.get(&anchor).ok_or(ToolProblem::AnchorNotIndexed { anchor })?;
        let vector =
            index.records.get(&candidate).ok_or(ToolProblem::AnchorNotIndexed { anchor: candidate })?;
        let similarity = cosine(anchor_embedding, vector).ok_or_else(|| {
            ToolProblem::Search(format!(
                "the vectors for {anchor:016x} and {candidate:016x} cannot be compared"
            ))
        })?;
        let anchor_features = features_of(index, anchor);
        let candidate_features = features_of(index, candidate);
        let judgement = super::filter::evaluate(&anchor_features, &candidate_features, &self.music);
        let breakdown = super::ensemble::breakdown(
            similarity,
            &candidate_features,
            &anchor_features,
            &judgement,
            &self.music,
        );
        Ok(Found {
            hash: candidate,
            score: breakdown.score(),
            similarity,
            breakdown,
            provenance: index.provenance.get(&candidate).cloned().unwrap_or_default(),
        })
    }
}

/// The text a category filter matches against, from a provenance source.
fn source_of(source: Option<&super::provenance::Source>) -> String {
    match source {
        Some(super::provenance::Source::Url(url)) => url.to_lowercase(),
        Some(super::provenance::Source::Pack(name)) => name.to_lowercase(),
        Some(super::provenance::Source::Unknown) | None => String::new(),
    }
}

/// The set of hashes an index holds, which is the identity a caller passes to a tool.
pub fn known(index: &Index) -> BTreeSet<u64> {
    index.records.keys().copied().collect()
}

/// **The index's feature maps as one lookup**, for a caller that has a hash and wants everything known about it.
pub fn features_by_hash(index: &Index) -> BTreeMap<u64, Features> {
    index.records.keys().map(|h| (*h, features_of(index, *h))).collect()
}

/// A stamp that matches a given index, for tests and for a caller that has only the index.
pub fn stamp_of(index: &Index) -> ModelStamp {
    index.stamp.clone()
}
