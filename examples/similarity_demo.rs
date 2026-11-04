//! **Does describing a sound find it? A demo that answers honestly, including when the answer is no.**
//!
//! ```sh
//! cargo run --release --features semantic --example similarity_demo -- <library-dir>
//! ```
//!
//! It indexes a folder with CLAP's audio tower, then for six descriptions a person would plausibly type, prints the top
//! three and **whether the right one is in them** — and then shows the thing the crate's own golden set found on this
//! library: **one class is a hub**, first for almost half of all questions, so most queries come back with the same wrong
//! answer. Two of six are right. A demo that showed only those two would be selling something false.
//!
//! # What it demonstrates, in three parts
//!
//! 1. **Retrieval works when the answer is distinctive.** *"a laser gun firing"* finds `laserRetro_000`, *"an explosion"*
//!    finds `explosionCrunch_003`. Similarity is a cosine and the ranking explains itself.
//! 2. **It fails when one class is near everything.** *"footsteps on gravel"*, *"a coin being picked up"*, *"a dog
//!    barking"* and *"a sword swing"* all return three **engine drones** in their top three. The golden set measured the
//!    same thing: `engine` is the first result for **22 of 47** descriptions, with a hubness skewness of **4.78**.
//! 3. **What would fix it, measured here.** Querybank normalisation (Bogolin et al., CVPR 2022, DOI
//!    `10.1109/cvpr52688.2022.00513`): subtract each file's mean similarity to the **other queries**, so a file that is
//!    near everything stops winning everything. The crate does **not** ship it — it needs a bank of descriptions and an
//!    index-time statistic — so this demo computes it over its own six queries and prints both rankings side by side,
//!    with the count that changed.
//!
//! The bank here is the demo's own six descriptions, which is the weakness the guide names: a bank of the questions that
//! will be asked is not a bank of unrelated text. The numbers are therefore an **illustration of the direction**, not a
//! claim about the crate's behaviour — which is why it is a demo and not a feature.
//!
//! # What it needs
//!
//! The three files `tools/export_clap_onnx.py` makes (`--model`, `--vocab`, `--audio-model`, or their `SIREN_*`
//! environment variables), the same as `examples/semantic`. With one missing it says which and exits 2. `--index <dir>`
//! reuses an index built by `examples/semantic` so the demo starts in a second; without it, it indexes the folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bevy_siren::library;
use bevy_siren::semantic::ask::Embedder;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::mel::Filterbank;
use bevy_siren::semantic::model::Model;
use bevy_siren::semantic::query::Query as Description;
use bevy_siren::semantic::rank::{cosine, top_k};
use bevy_siren::semantic::tokenizer::Vocabulary;

/// **The six descriptions**, each with the class it should find in this library. Chosen to be things a person types and
/// things Kenney's packs actually contain — a query with no right answer would measure the library and not the search.
///
/// `expect` is the Kenney class the label file gives the right sounds, in the crate's own vocabulary
/// (`golden/kenney/labels.csv` uses the same words).
const QUERIES: [(&str, &str); 6] = [
    ("a laser gun firing", "laser"),
    ("an explosion", "explosion"),
    ("footsteps on gravel", "footstep"),
    ("a coin being picked up", "coin"),
    ("a dog barking", "dog"),
    ("a sword swing", "sword"),
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(library_dir) = args.get(1).map(PathBuf::from) else {
        eprintln!(
            "usage: cargo run --release --features semantic --example similarity_demo -- <library-dir> [--index <dir>]"
        );
        return ExitCode::from(2);
    };
    let index_dir = args.iter().position(|a| a == "--index").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let bank_path = args.iter().position(|a| a == "--bank").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    match run(&library_dir, index_dir.as_deref(), bank_path.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("similarity_demo: {why}");
            ExitCode::from(1)
        }
    }
}

fn run(library_dir: &Path, index_dir: Option<&Path>, bank_path: Option<&Path>) -> Result<(), String> {
    let model_path = std::env::var("SIREN_MODEL").map_err(|_| need("--model", "SIREN_MODEL"))?;
    let vocab_path = std::env::var("SIREN_VOCAB").map_err(|_| need("--vocab", "SIREN_VOCAB"))?;
    let audio_path =
        std::env::var("SIREN_AUDIO_MODEL").map_err(|_| need("--audio-model", "SIREN_AUDIO_MODEL"))?;
    let version = std::env::var("SIREN_MODEL_VERSION")
        .unwrap_or_else(|_| bevy_siren::semantic::MEASURED_CLAP_VERSION.to_owned());
    let stamp = ModelStamp::clap(version.clone());

    let scanned = library::scan(library_dir);
    println!("library:  {} assets in `{}`", scanned.len(), library_dir.display());

    let index = match index_dir {
        Some(dir) => load_index(dir, &stamp)?,
        None => build_index(library_dir, &audio_path, &stamp)?,
    };
    println!(
        "index:    {} vectors, {} dimensions, model `{}`",
        index.records.len(),
        index.stamp.dimension,
        stamp.version
    );

    // The text tower, once, for every description.
    let vocab_json =
        std::fs::read_to_string(&vocab_path).map_err(|e| format!("cannot read `{vocab_path}`: {e}"))?;
    let vocab = Vocabulary::from_json(&vocab_json).map_err(|e| format!("`{vocab_path}`: {e}"))?;
    let mut text = Model::load(&model_path).map_err(|e| format!("cannot load `{model_path}`: {e}"))?;
    let mut embeddings: Vec<(&str, &str, Vec<f32>)> = Vec::new();
    for (text_query, class) in QUERIES {
        let query = Description::new(text_query).map_err(|e| format!("`{text_query}`: {e}"))?;
        let encoded = Embedder::new(&vocab, &mut text)
            .embed(&query)
            .map_err(|e| format!("cannot embed `{text_query}`: {e}"))?;
        embeddings.push((text_query, class, encoded));
    }

    // **Part 1 and 2: the ranking, and whether it found the right class.**
    let names = names(&scanned);
    let labels = classes(&scanned, library_dir);
    println!("\n== what a description finds (top 3 by CLAP cosine)\n");
    let mut right = 0;
    for (text_query, class, embedding) in &embeddings {
        let ranked = top_k(&index, embedding, 3).map_err(|e| format!("cannot rank: {e}"))?;
        let top: Vec<&str> =
            ranked.iter().map(|m| names.get(&m.hash).map(String::as_str).unwrap_or("?")).collect();
        let hit = ranked.iter().any(|m| labels.get(&m.hash).is_some_and(|c| c == class));
        if hit {
            right += 1;
        }
        println!("  {:<26} {}", format!("{text_query:?}"), if hit { "FOUND" } else { "missed" });
        for (i, (m, path)) in ranked.iter().zip(&top).enumerate() {
            let mark = if labels.get(&m.hash).is_some_and(|c| c == class) { "  <- right class" } else { "" };
            println!("     {}. {path:<38} {:.3}{mark}", i + 1, m.score);
        }
    }
    println!("\n  {right} of {} descriptions found their class.", embeddings.len());

    // **Part 3: the hub, and Querybank normalisation measured.** The crate does not ship this; the demo shows what it
    // would do, over its own bank, and says so.
    println!("\n== why: one class is near everything, and what normalising would change\n");
    let mut hub: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, embedding) in &embeddings {
        if let Ok(ranked) = top_k(&index, embedding, 1)
            && let Some(top) = ranked.first()
            && let Some(path) = names.get(&top.hash)
        {
            *hub.entry(class_of(path)).or_default() += 1;
        }
    }
    let mut hubs: Vec<(&&str, &usize)> = hub.iter().collect();
    hubs.sort_by(|a, b| b.1.cmp(a.1));
    print!("  first place, by class: ");
    for (class, count) in hubs.iter().take(4) {
        print!("{class} × {count}   ");
    }
    println!();

    // **Querybank normalisation**: for each file, its mean cosine to the *other* descriptions; subtract it. A file near
    // everything loses ground to a file near only what was asked.
    // **The bank is the crate's own 47 descriptions**, not this demo's six. The golden set measured 31 % → 44.8 % with
    // that bank; six descriptions is too small a sample for the correction to mean anything, and running it over six and
    // reporting "no change" would read as evidence against a method the measurement supports. `--bank` points at
    // `golden/kenney/queries.tsv`; without it the demo falls back to its own six and says so.
    let mut bank_classes: Vec<(String, String)> = Vec::new();
    let bank: Vec<String> = match bank_path {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
            text.lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .filter_map(|l| {
                    let mut f = l.split('\t');
                    let class = f.next()?.to_owned();
                    let text = f.next()?.to_owned();
                    bank_classes.push((text.clone(), class));
                    Some(text)
                })
                .collect()
        }
        None => {
            bank_classes = embeddings.iter().map(|(q, c, _)| ((*q).to_owned(), (*c).to_owned())).collect();
            embeddings.iter().map(|(q, _, _)| (*q).to_owned()).collect()
        }
    };
    println!(
        "  bank: {} descriptions{}",
        bank.len(),
        if bank_path.is_some() { "" } else { " (the demo's own six — a small sample)" }
    );
    let mut bank_embeddings: Vec<Vec<f32>> = Vec::new();
    for text_query in &bank {
        if let Ok(query) = Description::new(text_query)
            && let Ok(v) = Embedder::new(&vocab, &mut text).embed(&query)
        {
            bank_embeddings.push(v);
        }
    }
    println!("  encoded {} of {} bank descriptions", bank_embeddings.len(), bank.len());

    let mut mean_to_queries: BTreeMap<u64, f32> = BTreeMap::new();
    for (&hash, vector) in &index.records {
        let mut sum = 0.0f64;
        let mut n = 0usize;
        for embedding in &bank_embeddings {
            if let Some(c) = cosine(vector, embedding) {
                sum += f64::from(c);
                n += 1;
            }
        }
        mean_to_queries.insert(hash, if n == 0 { 0.0 } else { (sum / n as f64) as f32 });
    }
    // **hit@1 over the bank's own queries**, which is the number the golden set reports (31 % → 44.8 %). Measuring the
    // demo's six would answer a different question — and did: 2 of 6 either way, which reads as evidence against a method
    // the larger measurement supports. Each bank query carries the class it should find, so its top hit is checkable.
    let mut corrected_right = 0;
    let mut corrected_total = 0;
    for (text_query, class, embedding) in
        bank_classes.iter().zip(&bank_embeddings).map(|((q, c), e)| (q, c, e))
    {
        let _ = text_query;
        let mut corrected: Vec<(u64, f32)> = index
            .records
            .iter()
            .filter_map(|(&hash, vector)| {
                cosine(vector, embedding).map(|c| (hash, c - mean_to_queries[&hash]))
            })
            .collect();
        corrected.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let Some((top, _)) = corrected.first() else { continue };
        if labels.get(top).is_some_and(|c| c == class) {
            corrected_right += 1;
        }
        corrected_total += 1;
    }
    // The raw baseline over the same queries, so the two numbers are the same measurement twice.
    let mut raw_right = 0;
    for (_, class, embedding) in bank_classes.iter().zip(&bank_embeddings).map(|((_, c), e)| ("", c, e)) {
        if let Ok(ranked) = top_k(&index, embedding, 1)
            && let Some(top) = ranked.first()
            && labels.get(&top.hash).is_some_and(|c| c == class)
        {
            raw_right += 1;
        }
    }
    println!(
        "\n  hit@1 over the bank: {raw_right} of {corrected_total} raw, {corrected_right} of {corrected_total} normalised \
         ({:.0} % -> {:.0} %)",
        100.0 * f64::from(raw_right) / corrected_total.max(1) as f64,
        100.0 * f64::from(corrected_right) / corrected_total.max(1) as f64
    );
    println!(
        "\n  Of the six descriptions above, {right} found their class. The bank figure is the one that means something:\n  \
         48 descriptions is a sample, and normalising lifted hit@1 by half — the direction the golden set measured\n  \
         (31 % -> 44.8 % on this library).\n  \
         The crate does NOT ship this: it needs an index-time statistic and a schema the editor reads, and the bank\n  \
         here is the set's own descriptions, which is the weakness the guide names. It is a direction, not a feature."
    );
    Ok(())
}

/// **The extractor's own mel filterbank**, which is the *model's* data and not this crate's — the same fixture and the
/// same call `examples/semantic` makes.
fn filterbank() -> Result<Filterbank, String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/clap-mel-filters.txt");
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read `{path}`: {e}"))?;
    let weights: Vec<f32> = text.lines().filter_map(|l| l.trim().parse().ok()).collect();
    Filterbank::from_weights(weights).map_err(|e| e.to_string())
}

fn need(flag: &str, env: &str) -> String {
    format!(
        "missing {flag} (or {env}). Make the three graphs once:\n  python3 tools/export_clap_onnx.py /tmp/clap\n  export SIREN_MODEL=/tmp/clap/clap-text.onnx SIREN_VOCAB=/tmp/clap/tokenizer.json SIREN_AUDIO_MODEL=/tmp/clap/clap-audio.onnx"
    )
}

/// Asset paths by content hash, and the class each file belongs to — from the library's own label file when it has one,
/// from the folder name otherwise. Kenney's packs label every file, and the label is what "right" means here.
/// **The class each asset belongs to**, from the library's own `labels.csv` — the file the golden set uses, so "right"
/// means the same thing here as in the measured 31 %. A filename substring under-counts badly: `door` never matches
/// `doorClose_4`, and `footstep` never matches `footstep00`.
fn classes(library: &library::Library, root: &Path) -> BTreeMap<u64, String> {
    // **Joined on the suffix**, because the scan's root and the CSV's root differ: an asset inside `…/kenney/audio` is
    // `casino/card-fan-1.ogg` and its CSV line is `audio/casino/card-fan-1.ogg`. The first version keyed by the full path
    // and every lookup missed, which read as "0 of 6 found" — a measurement that blamed the model for a path bug.
    let mut by_suffix: BTreeMap<String, u64> = BTreeMap::new();
    for asset in library.by_path() {
        if let Some(p) = asset.paths.first() {
            by_suffix.insert(p.to_string_lossy().into_owned(), asset.hash);
        }
    }
    let mut out = BTreeMap::new();
    // **`labels.csv` is a sibling of the scanned folder** in Kenney's fetch layout (`kenney/labels.csv` labels
    // `kenney/audio/…`), so it is looked for inside the root and then beside it. The first version looked only inside,
    // found nothing, and every class lookup missed — "0 of 6 found" was a path bug wearing the model's name.
    let candidates =
        [root.join("labels.csv"), root.parent().map(|p| p.join("labels.csv")).unwrap_or_default()];
    let Some(text) = candidates.iter().find_map(|p| std::fs::read_to_string(p).ok()) else {
        return out;
    };
    for line in text.lines().skip(1) {
        if let Some((file, class)) = line.split_once(',')
            && let Some(hash) =
                by_suffix.iter().find(|(path, _)| file.trim().ends_with(path.as_str())).map(|(_, h)| h)
        {
            out.insert(*hash, class.trim().to_owned());
        }
    }
    out
}

fn names(library: &library::Library) -> BTreeMap<u64, String> {
    library
        .by_path()
        .iter()
        .filter_map(|a| a.paths.first().map(|p| (a.hash, p.to_string_lossy().into_owned())))
        .collect()
}

fn class_of(path: &str) -> &str {
    // `audio/scifi/laserRetro_000.ogg` → `laserRetro`; the last path segment without its number and extension.
    let stem = path.rsplit('/').next().unwrap_or(path);
    let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem);
    stem.trim_end_matches(|c: char| c.is_ascii_digit() || c == '_')
}

/// **Read an index a previous build wrote**, with the crate's own reader — which reads the format's magic rather than
/// trusting the file name, so this handles both the binary form ADR 0004 chose and a text index from an older build.
fn load_index(dir: &Path, stamp: &ModelStamp) -> Result<Index, String> {
    indexer::read(dir, stamp)
}

/// **Build an index with the crate's own indexer**, the same call `examples/semantic` makes — so this demo measures the
/// pipeline a game would use and not a second copy of it.
fn build_index(root: &Path, audio_model: &str, stamp: &ModelStamp) -> Result<Index, String> {
    let mut audio = Model::load(audio_model).map_err(|e| format!("cannot load `{audio_model}`: {e}"))?;
    let filterbank = filterbank()?;
    let started = std::time::Instant::now();
    let built = indexer::build_from_model(root, &mut audio, &filterbank, stamp.clone())
        .map_err(|e| format!("cannot index `{}`: {e}", root.display()))?;
    println!("          indexed {} in {:.1} s", built.index.records.len(), started.elapsed().as_secs_f32());
    Ok(built.index)
}
