//! **`#35`: the golden set, run on the real model** — recall, precision and the floor, measured on recordings people labelled.
//!
//! ```sh
//! python3 tools/fetch_esc50.py /tmp/esc50      # once: 2 000 clips, every one verified against golden/esc50/manifest.sha256
//! python3 tools/export_clap_onnx.py /tmp/clap  # once: the two graphs and the tokenizer
//! cargo run --release --features semantic --example goldenbench -- /tmp/esc50 \
//!     --model /tmp/clap/clap-text.onnx --vocab /tmp/clap/tokenizer.json --audio-model /tmp/clap/clap-audio.onnx \
//!     --model-version "$(cat /tmp/clap/clap-export.txt)"
//! ```
//!
//! The model flags also read `SIREN_MODEL`, `SIREN_VOCAB`, `SIREN_AUDIO_MODEL` and `SIREN_MODEL_VERSION`, as `examples/semantic` does.
//!
//! # What it prints
//!
//! 1. **The paper's zero-shot accuracy**, from the same embeddings: each clip against one prompt per class
//!    (`"This is a sound of <class>."`), top-1 — the number the model's paper reports for this dataset (88.0 – 91.0 % on the
//!    overlap-excluded subset). It tests the *pipeline* against a published figure.
//! 2. **Text-to-audio retrieval**, which is what the crate is for: each description against every clip. Hit@1/3/10, precision@10,
//!    MRR, mAP@10, and the queries that missed, by name.
//! 3. **The scores**: the cosines of the right answers and of the wrong ones, and the best score of each query that should find
//!    nothing.
//! 4. **The floor curve**: what each floor keeps of the right answers and what it silences of the wrong ones.
//! 5. That the **tool path** (`tools::search_music`, what `siren-tools` answers) returns the same ten as the raw ranking.
//! 6. A **baseline** compare — `--bless` records one. It is refused when the set or the model differ from the baseline's, because a
//!    metric compared against a baseline from a different corpus is a number about nothing.
//!
//! With the model files or the library missing it says exactly which and exits 2, before it reads, builds or writes anything.
//! `--dump <file>` writes every query's score against every clip, for a cross-check outside this crate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use bevy_siren::library::{self, Library};
use bevy_siren::semantic::ask::Embedder;
use bevy_siren::semantic::eval::{self, FloorRow, ScoredCase};
use bevy_siren::semantic::golden::GoldenSet;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::mel::Filterbank;
use bevy_siren::semantic::model::{Device, Model};
use bevy_siren::semantic::query::Query;
use bevy_siren::semantic::rank::{self, Floor};
use bevy_siren::semantic::search::Ranked;
use bevy_siren::semantic::tokenizer::Vocabulary;
use bevy_siren::semantic::tools::{self, Toolset};

/// The floors the curve is read at. A **design default** that spans the range cross-modal cosines occupy — CLIPScore reports
/// *"roughly zero to roughly .4"* for image–text — with a margin below zero to show what a negative cosine is, and above 0.4.
const FLOORS: [f32; 13] = [-0.10, -0.05, 0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50];

/// How far a metric may fall below the baseline before it is a regression. A **design default**: a hit rate over 50 queries moves
/// in steps of 0.02, so a real regression is at least one query, and this only absorbs floating-point differences between machines.
const TOLERANCE: f32 = 0.005;

/// The prompt the paper turns a class label into (`10.1109/icassp49357.2023.10095969`, §zero-shot classification).
fn prompt(class: &str) -> String {
    format!("This is a sound of {}.", class.replace('_', " "))
}

struct Options {
    library: PathBuf,
    queries: PathBuf,
    text_model: Option<PathBuf>,
    vocab: Option<PathBuf>,
    audio_model: Option<PathBuf>,
    version: Option<String>,
    bless: bool,
    dump: Option<PathBuf>,
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn usage() -> &'static str {
    "usage: goldenbench <library-dir> [--model clap-text.onnx --vocab tokenizer.json] [--audio-model clap-audio.onnx]\n\
     \x20               [--model-version <v>] [--queries golden/esc50/queries.tsv] [--bless] [--dump <file>]\n\n\
     <library-dir> is what `python3 tools/fetch_esc50.py <library-dir>` made. Each model flag also reads SIREN_MODEL, SIREN_VOCAB,\n\
     SIREN_AUDIO_MODEL and SIREN_MODEL_VERSION; the files are made once by `python3 tools/export_clap_onnx.py <dir>`."
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        library: PathBuf::new(),
        queries: repo("golden/esc50/queries.tsv"),
        text_model: env("SIREN_MODEL").map(PathBuf::from),
        vocab: env("SIREN_VOCAB").map(PathBuf::from),
        audio_model: env("SIREN_AUDIO_MODEL").map(PathBuf::from),
        version: env("SIREN_MODEL_VERSION"),
        bless: false,
        dump: None,
    };
    let mut positional = Vec::new();
    let mut args = args;
    while let Some(arg) = args.next() {
        let mut value = |what: &str| args.next().ok_or_else(|| format!("{arg} needs {what}"));
        match arg.as_str() {
            "--model" => options.text_model = Some(PathBuf::from(value("a file")?)),
            "--vocab" => options.vocab = Some(PathBuf::from(value("a file")?)),
            "--audio-model" => options.audio_model = Some(PathBuf::from(value("a file")?)),
            "--model-version" => options.version = Some(value("a version string")?),
            "--queries" => options.queries = PathBuf::from(value("a file")?),
            "--dump" => options.dump = Some(PathBuf::from(value("a file")?)),
            "--bless" => options.bless = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            other => positional.push(other.to_owned()),
        }
    }
    let [library] = <[String; 1]>::try_from(positional).map_err(|got| {
        format!("one argument is required — the library directory — and {} were given", got.len())
    })?;
    options.library = PathBuf::from(library);
    Ok(options)
}

fn seconds(since: Instant) -> f64 {
    since.elapsed().as_secs_f64()
}

fn percent(share: f32) -> String {
    format!("{:5.1} %", share * 100.0)
}

/// The extractor's own mel filterbank, which is the *model's* data: copied from `ClapFeatureExtractor.mel_filters` and checked in
/// for the tests. An example runs from a checkout, so it reads it from there.
fn filterbank() -> Result<Filterbank, String> {
    let path = repo("tests/fixtures/clap-mel-filters.txt");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    let weights: Vec<f32> = text.lines().filter_map(|l| l.trim().parse().ok()).collect();
    Filterbank::from_weights(weights).map_err(|e| e.to_string())
}

/// The first line of a message, cut to a screenful: the runtime's own errors run to several lines.
fn short(message: &str) -> String {
    let line = message.lines().next().unwrap_or("");
    match line.char_indices().nth(140) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_owned(),
    }
}

/// The set's identity: the commit its manifest pins, a hash of the manifest's bytes, and a hash of the queries it was asked — so a
/// baseline says which set *and which questions* it is about. A different queries file is a different measurement.
fn set_identity(queries: &str) -> Result<String, String> {
    let path = repo("golden/esc50/manifest.sha256");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    let commit = text
        .lines()
        .find_map(|line| line.strip_prefix("# commit:"))
        .map(str::trim)
        .ok_or_else(|| format!("`{}` names no commit", path.display()))?;
    Ok(format!(
        "esc50 @ {} manifest {:016x} queries {:016x}",
        &commit[..commit.len().min(8)],
        library::fnv1a(text.as_bytes()),
        library::fnv1a(queries.as_bytes())
    ))
}

/// What a clip is labelled as, for the zero-shot score: by class, the clips whose content is in it.
struct ZeroShot {
    correct: usize,
    total: usize,
    by_class: BTreeMap<String, (usize, usize)>,
    confused: BTreeMap<(String, String), usize>,
}

fn zero_shot(
    index: &Index,
    prompts: &BTreeMap<String, Vec<f32>>,
    by_class: &BTreeMap<String, BTreeSet<u64>>,
) -> ZeroShot {
    let mut truth: BTreeMap<u64, BTreeSet<&str>> = BTreeMap::new();
    for (class, hashes) in by_class {
        for hash in hashes {
            truth.entry(*hash).or_default().insert(class);
        }
    }
    let mut result = ZeroShot { correct: 0, total: 0, by_class: BTreeMap::new(), confused: BTreeMap::new() };
    for (hash, vector) in &index.records {
        let Some(classes) = truth.get(hash) else { continue };
        let best = prompts
            .iter()
            .filter_map(|(class, p)| rank::cosine(p, vector).map(|score| (class, score)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let Some((predicted, _)) = best else { continue };
        let right = classes.contains(predicted.as_str());
        result.total += 1;
        result.correct += usize::from(right);
        for class in classes {
            let entry = result.by_class.entry((*class).to_string()).or_default();
            entry.1 += 1;
            entry.0 += usize::from(right);
            if !right {
                *result.confused.entry(((*class).to_string(), predicted.clone())).or_default() += 1;
            }
        }
    }
    result
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("goldenbench: {why}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };
    // **Refused before anything is loaded**: the baseline is for the default queries, and recording another file's numbers over it
    // would make every later compare a compare against the wrong questions.
    let default_queries = options.queries == repo("golden/esc50/queries.tsv");
    if options.bless && !default_queries {
        eprintln!(
            "goldenbench: the baseline is for golden/esc50/queries.tsv, and `--bless` will not record another queries file's numbers over it"
        );
        return ExitCode::from(2);
    }
    let stamp = match &options.version {
        Some(version) => ModelStamp::clap(version.as_str()),
        None => ModelStamp::measured_clap(),
    };
    let root = options.library.as_path();

    // 1. The set and the library — nothing is loaded, built or written before these agree.
    if !root.is_dir() {
        eprintln!("goldenbench: `{}` is not a directory\n\n{}", root.display(), usage());
        return ExitCode::from(2);
    }
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))
    };
    let (labels, queries) = match (read(&root.join("labels.csv")), read(&options.queries)) {
        (Ok(labels), Ok(queries)) => (labels, queries),
        (Err(why), _) | (_, Err(why)) => {
            eprintln!(
                "goldenbench: {why}\n\n`labels.csv` is written by `python3 tools/fetch_esc50.py {}`, which also downloads and verifies the clips.",
                root.display()
            );
            return ExitCode::from(2);
        }
    };
    let set = match GoldenSet::parse(&labels, &queries) {
        Ok(set) => set,
        Err(why) => {
            eprintln!("goldenbench: the golden set is not loadable — {why}");
            return ExitCode::from(2);
        }
    };
    let identity = match set_identity(&queries) {
        Ok(identity) => identity,
        Err(why) => {
            eprintln!("goldenbench: {why}");
            return ExitCode::from(2);
        }
    };
    let library: Library = library::scan(root);
    let (absent, stray) = (set.absent(&library), set.unlabelled(&library));
    if !absent.is_empty() || !stray.is_empty() {
        eprintln!(
            "goldenbench: the library is not the set — {} labelled files are absent{}, and {} files nobody labelled are there{}.\n\
             A partial or padded copy measures a different set, so nothing was run. `python3 tools/fetch_esc50.py {}` repairs it.",
            absent.len(),
            absent.first().map(|f| format!(" (the first is `{f}`)")).unwrap_or_default(),
            stray.len(),
            stray.first().map(|f| format!(" (the first is `{}`)", f.display())).unwrap_or_default(),
            root.display()
        );
        return ExitCode::from(2);
    }

    // 2. What it needs, said before anything is loaded.
    let reusable = match indexer::read(root, &stamp) {
        Ok(index) if index.records.keys().copied().eq(library.assets.keys().copied()) => Some(index),
        _ => None,
    };
    let mut missing = Vec::new();
    let mut need = |flag: &str, what: &str, path: &Option<PathBuf>| match path {
        None => missing.push(format!("  {flag:<14} {what} — not given")),
        Some(p) if !p.is_file() => {
            missing.push(format!("  {flag:<14} {what} — `{}` does not exist", p.display()))
        }
        Some(_) => {}
    };
    need("--model", "the CLAP text graph, clap-text.onnx", &options.text_model);
    need("--vocab", "its tokenizer.json", &options.vocab);
    if reusable.is_none() {
        need(
            "--audio-model",
            "the CLAP audio graph, clap-audio.onnx (the library has to be indexed)",
            &options.audio_model,
        );
    }
    if !missing.is_empty() {
        eprintln!(
            "goldenbench: cannot run — {} file(s) missing:\n{}\n\nMake them once with `python3 tools/export_clap_onnx.py <dir>`.",
            missing.len(),
            missing.join("\n")
        );
        return ExitCode::from(2);
    }
    let (Some(text_path), Some(vocab_path)) = (&options.text_model, &options.vocab) else {
        return ExitCode::from(2);
    };

    let positives = set.queries.iter().filter(|q| q.class.is_some()).count();
    println!(
        "goldenbench: {identity} — {} clips, {} classes, {positives} queries and {} that should find nothing",
        library.len(),
        set.classes().len(),
        set.queries.len() - positives
    );
    println!("             {}; model {}", Device::available().describe(), stamp.version);

    // 3. The index: reused when it is current, built with the audio tower — every clip, or it stops — when it is not.
    let index = match reusable {
        Some(index) => {
            println!(
                "index:       {} clips, read from `{}` — current, so no audio graph was loaded",
                index.len(),
                indexer::INDEX_FILE
            );
            index
        }
        None => {
            let Some(audio_path) = &options.audio_model else { return ExitCode::from(2) };
            let mut audio = match Model::load(&audio_path.to_string_lossy()) {
                Ok(model) => model,
                Err(why) => {
                    eprintln!("goldenbench: cannot load the audio graph: {why}");
                    return ExitCode::from(1);
                }
            };
            let filterbank = match filterbank() {
                Ok(filterbank) => filterbank,
                Err(why) => {
                    eprintln!("goldenbench: {why}");
                    return ExitCode::from(1);
                }
            };
            println!(
                "index:       indexing {} clips with the real audio tower (about 60 ms a clip) …",
                library.len()
            );
            let started = Instant::now();
            let built = match indexer::build_from_model(root, &mut audio, &filterbank, stamp.clone()) {
                Ok(built) => built,
                Err(why) => {
                    eprintln!("goldenbench: cannot build the index: {why}");
                    return ExitCode::from(1);
                }
            };
            let took = seconds(started);
            println!(
                "             {} in {took:.1} s ({:.1} clips/s); {} licences attached",
                short(&built.describe()),
                built.indexed() as f64 / took.max(1e-9),
                built.index.provenance.values().filter(|p| p.is_recorded()).count()
            );
            // **Every clip or nothing**: a partial index measures a different set, and the numbers would be about whichever clips
            // happened to embed.
            if !built.failed.is_empty() || built.indexed() != library.len() {
                for failure in built.failed.iter().take(5) {
                    eprintln!(
                        "goldenbench: could not index `{}`: {}",
                        failure.path.display(),
                        short(&failure.why)
                    );
                }
                eprintln!(
                    "goldenbench: {} of {} clips were indexed, so nothing was measured and no index was written",
                    built.indexed(),
                    library.len()
                );
                return ExitCode::from(1);
            }
            match indexer::write(root, &built.index) {
                Ok(path) => println!("             written to `{}`", path.display()),
                Err(why) => eprintln!("goldenbench: the index was built but not saved: {why}"),
            }
            built.index
        }
    };

    // **What the licence lines say**, read from the index the sidecar was joined into: the crate's own judgement of each recorded
    // licence, with *unknown* kept apart from *no* — a licence it does not recognise is never presented as permissive.
    let (mut permit, mut forbid, mut unknown) = (0usize, 0usize, 0usize);
    for hash in index.records.keys() {
        match index.provenance.get(hash).and_then(|p| p.permits_commercial_use()) {
            Some(true) => permit += 1,
            Some(false) => forbid += 1,
            None => unknown += 1,
        }
    }
    println!(
        "licences:    {permit} permit commercial use, {forbid} do not, {unknown} unknown or not recognised — a person decides those"
    );

    // 4. Embed every description and every class prompt with the text tower.
    let started = Instant::now();
    let vocabulary =
        match read(vocab_path).and_then(|text| Vocabulary::from_json(&text).map_err(|e| e.to_string())) {
            Ok(vocabulary) => vocabulary,
            Err(why) => {
                eprintln!("goldenbench: cannot read the vocabulary: {why}");
                return ExitCode::from(1);
            }
        };
    let mut text = match Model::load(&text_path.to_string_lossy()) {
        Ok(model) => model,
        Err(why) => {
            eprintln!("goldenbench: cannot load the text graph: {why}");
            return ExitCode::from(1);
        }
    };
    let mut embed = |words: &str| -> Result<(Query, Vec<f32>), String> {
        let query = Query::new(words).map_err(|e| e.to_string())?;
        let embedding = Embedder::new(&vocabulary, &mut text).embed(&query).map_err(|e| e.to_string())?;
        Ok((query, embedding))
    };
    let mut asked: Vec<(Query, Vec<f32>)> = Vec::new();
    for query in &set.queries {
        match embed(&query.description) {
            Ok(pair) => asked.push(pair),
            Err(why) => {
                eprintln!("goldenbench: cannot embed `{}`: {why}", query.description);
                return ExitCode::from(1);
            }
        }
    }
    let mut prompts: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    for class in set.classes() {
        match embed(&prompt(class)) {
            Ok((_, embedding)) => {
                prompts.insert(class.to_string(), embedding);
            }
            Err(why) => {
                eprintln!("goldenbench: cannot embed the prompt for `{class}`: {why}");
                return ExitCode::from(1);
            }
        }
    }
    println!(
        "text:        {} descriptions and {} prompts embedded in {:.1} s",
        asked.len(),
        prompts.len(),
        seconds(started)
    );

    // 5. Rank every clip for every query, with the crate's own `top_k` — the first step of every search the crate answers.
    let by_class = set.right_answers(&library);
    let started = Instant::now();
    let mut cases: Vec<ScoredCase> = Vec::new();
    for (query, (_, embedding)) in set.queries.iter().zip(&asked) {
        let ranked = match rank::top_k(&index, embedding, index.len()) {
            Ok(ranked) => ranked,
            Err(why) => {
                eprintln!("goldenbench: cannot rank `{}`: {why}", query.description);
                return ExitCode::from(1);
            }
        };
        cases.push(ScoredCase {
            description: query.description.clone(),
            right: query.class.as_ref().and_then(|class| by_class.get(class)).cloned().unwrap_or_default(),
            scored: ranked.iter().map(|m| (m.hash, m.score)).collect(),
        });
    }
    println!(
        "ranking:     {} queries over {} clips in {:.1} ms\n",
        cases.len(),
        index.len(),
        seconds(started) * 1000.0
    );

    // ---- 1. zero-shot
    let zs = zero_shot(&index, &prompts, &by_class);
    let zs_accuracy = zs.correct as f32 / zs.total.max(1) as f32;
    println!(
        "1. Zero-shot classification — the paper's protocol: each clip against \"This is a sound of <class>.\", top-1"
    );
    println!(
        "   accuracy {}   ({} of {} clips)      paper: 88.0 – 91.0 % on the overlap-excluded subset; people score 81.3 %",
        percent(zs_accuracy),
        zs.correct,
        zs.total
    );
    let mut weakest: Vec<(&String, &(usize, usize))> = zs.by_class.iter().collect();
    weakest.sort_by(|a, b| {
        (a.1.0 as f32 / a.1.1.max(1) as f32).total_cmp(&(b.1.0 as f32 / b.1.1.max(1) as f32))
    });
    println!(
        "   weakest classes: {}",
        weakest
            .iter()
            .take(5)
            .map(|(class, (right, total))| format!(
                "{class} {:.0} %",
                *right as f32 / (*total).max(1) as f32 * 100.0
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut confused: Vec<(&(String, String), &usize)> = zs.confused.iter().collect();
    confused.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    println!(
        "   most confused:   {}\n",
        confused
            .iter()
            .take(5)
            .map(|((from, to), n)| format!("{from} → {to} ×{n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // ---- 2. retrieval
    let scores = eval::score_cases(&cases);
    println!(
        "2. Text-to-audio retrieval — {} descriptions, every clip ranked, a right answer is any clip of the class",
        scores.queries
    );
    println!(
        "   hit@1 {}   hit@3 {}   hit@10 {}   precision@10 {}   MRR {:.3}   mAP@10 {:.3}",
        percent(scores.hit_1),
        percent(scores.hit_3),
        percent(scores.hit_10),
        percent(scores.precision_10),
        scores.mrr,
        scores.map_10
    );
    if scores.missed.is_empty() {
        println!("   every description found a right answer within the first {}", eval::REPORT_K);
    } else {
        println!(
            "   no right answer in the first {}: {}",
            eval::REPORT_K,
            scores
                .missed
                .iter()
                .map(|(d, at)| match at {
                    Some(rank) => format!("`{d}` (first right answer at {rank})"),
                    None => format!("`{d}` (none ranked)"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let mut worst: Vec<&ScoredCase> = cases.iter().filter(|c| !c.is_negative() && !c.hit(1)).collect();
    worst.sort_by(|a, b| {
        a.precision(10).total_cmp(&b.precision(10)).then_with(|| a.description.cmp(&b.description))
    });
    if !worst.is_empty() {
        println!(
            "   best result wrong, by precision@10: {}",
            worst
                .iter()
                .take(8)
                .map(|c| format!("`{}` {:.0} %", c.description, c.precision(10) * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();

    // ---- 3. bands
    println!("3. Scores — CLAP cosines, before any weighting");
    println!(
        "   {:<26} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8}",
        "", "min", "p05", "p25", "median", "p75", "p95", "max", "n"
    );
    let row = |name: &str, b: eval::ScoreBand| {
        println!(
            "   {name:<26} {:>+7.3} {:>+7.3} {:>+7.3} {:>+7.3} {:>+7.3} {:>+7.3} {:>+7.3} {:>8}",
            b.min, b.p05, b.p25, b.median, b.p75, b.p95, b.max, b.n
        )
    };
    let bands = eval::score_bands(&cases);
    if let Some(bands) = &bands {
        row("right answers", bands.right);
        row("wrong answers", bands.wrong);
        if let Some(best) = bands.negative_best {
            row("best of the no-answer set", best);
        }
    }
    // **Which sound each no-answer query found instead.** The score is half the story: a query that should find nothing and finds a
    // cat is usually the confound its rationale names — a lion is not in this corpus, but it is not unlike a cat.
    let mut found: Vec<(f32, &str, &str)> = cases
        .iter()
        .filter(|c| c.is_negative())
        .filter_map(|c| {
            let (hash, score) = c.scored.first()?;
            let class = library
                .assets
                .get(hash)
                .and_then(|a| a.paths.first())
                .and_then(|p| set.label_of(p))
                .unwrap_or("?");
            Some((*score, c.description.as_str(), class))
        })
        .collect();
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !found.is_empty() {
        println!("   the queries that should find nothing, by the best clip they found:");
        for (score, description, class) in &found {
            println!("     {description:<36} {score:>+6.3}  {class}");
        }
    }
    println!();

    // ---- 4. floor
    let curve: Vec<FloorRow> = eval::floor_curve(&cases, &FLOORS);
    println!("4. The floor — what each keeps of the right answers and silences of the wrong ones");
    println!(
        "   {:>6}  {:>10}  {:>10}  {:>9}  {:>10}  {:>10}",
        "floor", "top-1 kept", "hit@3 kept", "returned", "precision", "silent"
    );
    for r in &curve {
        println!(
            "   {:>+6.2}  {:>10}  {:>10}  {:>9.1}  {:>10}  {:>10}",
            r.floor,
            percent(r.top_1_kept),
            percent(r.hit_3_kept),
            r.returned,
            r.precision.map_or_else(|| String::from("—"), percent),
            r.silent.map_or_else(|| String::from("—"), percent)
        );
    }
    println!(
        "   (top-1 kept: descriptions whose best result is right and clears the floor; hit@3 kept: a right answer among the first three"
    );
    println!(
        "    that clear it; returned: results per query that clear the floor, of {}; precision: the share of those that are right;",
        index.len()
    );
    println!(
        "    silent: the share of the {} no-answer queries that return nothing)\n",
        cases.iter().filter(|c| c.is_negative()).count()
    );

    // ---- 5. the tool path
    let features = tools::features_by_hash(&index);
    let toolset = Toolset::default();
    let unanchored = (0u64..).find(|h| !index.records.contains_key(h)).unwrap_or(0);
    let (mut agree, mut tried) = (0usize, 0usize);
    for (case, (query, embedding)) in cases.iter().zip(&asked) {
        if case.is_negative() {
            continue;
        }
        tried += 1;
        let request = Ranked {
            embedding,
            anchor: unanchored,
            features: &features,
            k: 10,
            floor: Floor { min_score: -1.0 },
            weights: &toolset.music,
        };
        if let Ok(answer) = tools::search_music(&index, &stamp, query, &request) {
            let same = answer.results.len() == 10
                && answer
                    .results
                    .iter()
                    .zip(&case.scored)
                    .all(|(found, (hash, score))| found.hash == *hash && found.similarity == *score);
            agree += usize::from(same);
        }
    }
    println!(
        "5. The tool path — `search_music` returns the same ten, with the same similarities, as the raw ranking: {agree} of {tried} queries\n"
    );

    // ---- 6. the human half
    println!(
        "6. Anchor judgements — none yet. `#35`'s third criterion (at least 50 pairs, each judged by a person with a reason) is a"
    );
    println!(
        "   person's listening; nothing here pretends otherwise. `docs/guide/golden-set.md` says how to add them.\n"
    );

    if let Some(path) = &options.dump {
        let names: Vec<(u64, String)> = index
            .records
            .keys()
            .map(|h| {
                (
                    *h,
                    library
                        .assets
                        .get(h)
                        .and_then(|a| a.paths.first())
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                )
            })
            .collect();
        let mut out = String::from("description");
        for (_, name) in &names {
            out.push('\t');
            out.push_str(name);
        }
        out.push('\n');
        for (case, (_, embedding)) in cases.iter().zip(&asked) {
            let by_hash: BTreeMap<u64, f32> = case.scored.iter().copied().collect();
            out.push_str(&case.description);
            for (hash, _) in &names {
                out.push_str(&format!("\t{:.6}", by_hash.get(hash).copied().unwrap_or(f32::NAN)));
            }
            out.push('\n');
            let _ = embedding;
        }
        for (class, embedding) in &prompts {
            out.push_str(&prompt(class));
            for (hash, _) in &names {
                let score =
                    index.records.get(hash).and_then(|v| rank::cosine(embedding, v)).unwrap_or(f32::NAN);
                out.push_str(&format!("\t{score:.6}"));
            }
            out.push('\n');
        }
        match std::fs::write(path, out) {
            Ok(()) => println!(
                "dump:        {} rows × {} clips written to `{}`\n",
                cases.len() + prompts.len(),
                names.len(),
                path.display()
            ),
            Err(why) => eprintln!("goldenbench: cannot write `{}`: {why}", path.display()),
        }
    }

    // ---- the baseline
    let metrics: [(&str, f32); 7] = [
        ("zero_shot", zs_accuracy),
        ("hit_1", scores.hit_1),
        ("hit_3", scores.hit_3),
        ("hit_10", scores.hit_10),
        ("precision_10", scores.precision_10),
        ("mrr", scores.mrr),
        ("map_10", scores.map_10),
    ];
    let baseline_path = repo("golden/esc50/baseline.txt");
    let model = stamp.to_string();
    if options.bless {
        let mut text = String::from(
            "# goldenbench's baseline. Regenerate with `cargo run --release --features semantic --example goldenbench -- <dir> --bless`.\n\
             # The set and the model are recorded because a metric compared against a baseline from a different corpus or a\n\
             # different model is a number about nothing (`#35`'s own pitfall, and `#45`'s).\n",
        );
        text.push_str(&format!("set: {identity}\nmodel: {model}\n"));
        for (name, value) in metrics {
            text.push_str(&format!("{name}: {value:.6}\n"));
        }
        return match std::fs::write(&baseline_path, text) {
            Ok(()) => {
                println!("baseline written to {}", baseline_path.display());
                ExitCode::SUCCESS
            }
            Err(why) => {
                eprintln!("goldenbench: cannot write the baseline: {why}");
                ExitCode::from(1)
            }
        };
    }
    let Ok(recorded) = std::fs::read_to_string(&baseline_path) else {
        println!("no baseline at {}: run with `--bless` to record one", baseline_path.display());
        return ExitCode::SUCCESS;
    };
    let field = |key: &str| recorded.lines().find_map(|l| l.strip_prefix(key)).map(|v| v.trim().to_string());
    let (Some(set_was), Some(model_was)) = (field("set:"), field("model:")) else {
        eprintln!("goldenbench: the baseline at {} is not readable", baseline_path.display());
        return ExitCode::from(1);
    };
    if set_was != identity || model_was != model {
        println!(
            "baseline:    recorded for `{set_was}` and `{model_was}`; this run is `{identity}` and `{model}`."
        );
        if !default_queries {
            println!(
                "             This run asked a different queries file, so nothing is compared; the baseline is for golden/esc50/queries.tsv."
            );
            return ExitCode::SUCCESS;
        }
        println!(
            "             Those are different measurements, so nothing is compared. `--bless` records this one."
        );
        return ExitCode::from(1);
    }
    let mut regressed = Vec::new();
    println!("baseline:    {:<14} {:>9} {:>9} {:>9}", "", "recorded", "now", "change");
    for (name, now) in metrics {
        let Some(was) = field(&format!("{name}:")).and_then(|v| v.parse::<f32>().ok()) else {
            eprintln!("goldenbench: the baseline has no `{name}`");
            return ExitCode::from(1);
        };
        let change = if (now - was).abs() < 5e-5 { 0.0 } else { now - was }; // not "-0.0000" for a difference of 1e-8
        println!("             {name:<14} {was:>9.4} {now:>9.4} {change:>+9.4}");
        if now < was - TOLERANCE {
            regressed.push(name);
        }
    }
    if regressed.is_empty() {
        println!("no regression (a fall of more than {TOLERANCE} is one)");
        ExitCode::SUCCESS
    } else {
        println!("REGRESSION: {} fell below the baseline", regressed.join(", "));
        ExitCode::from(1)
    }
}
