//! **`#38`: index a folder, describe a mood, hear the result.**
//!
//! ```sh
//! cargo run --release --features semantic --example semantic -- <library-dir> "<description>"
//! ```
//!
//! It scans the folder, builds (or reuses) the index CLAP's audio tower makes of it, embeds your description with CLAP's text
//! tower, prints the top results **with the reason each one ranked** and where it came from, and plays one through a bus —
//! the same call `search_music` answers over MCP, and the same `Play` a game would send, so the voice budget, the bus and the
//! log apply and the answer is `Played` or `Refused` like any other.
//!
//! # What it needs, and what it says when it is missing
//!
//! The crate does not ship the model. Three files, made once by `tools/export_clap_onnx.py`:
//!
//! | flag | environment | what |
//! |---|---|---|
//! | `--model` | `SIREN_MODEL` | the text graph, `clap-text.onnx` |
//! | `--vocab` | `SIREN_VOCAB` | its `tokenizer.json` |
//! | `--audio-model` | `SIREN_AUDIO_MODEL` | the audio graph, `clap-audio.onnx` — **only when the folder needs indexing** |
//!
//! With one missing it prints **exactly which**, and how to make them, and exits with status 2 before it reads, writes or
//! plays anything. **There is no GPU path**: this build runs the model on the CPU in fp32 and says so, so *"no GPU"* is
//! not a failure here; a missing model is.
//!
//! Also: `--model-version` (`SIREN_MODEL_VERSION`) names the export that made the vectors — the line in `clap-export.txt` — and
//! defaults to the one this crate measured; `--k` (default 3), `--pick` (which result to play, default 1), `--bus` (default
//! `Music`) and `--no-play`, which prints the ranking and exits without opening an audio device.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use bevy::app::ScheduleRunnerPlugin;
use bevy::audio::{AudioPlugin, AudioSink, AudioSource};
use bevy::prelude::*;
use bevy_siren::library::{self, Library};
use bevy_siren::semantic::ask::Embedder;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::mel::Filterbank;
use bevy_siren::semantic::model::{Device, Model};
use bevy_siren::semantic::query::Query as Description;
use bevy_siren::semantic::rank::Floor;
use bevy_siren::semantic::search::Ranked;
use bevy_siren::semantic::tokenizer::Vocabulary;
use bevy_siren::semantic::tools::{self, Found, Toolset};
use bevy_siren::{CueTable, Play, Played, Refused, SirenPlugin, SirenSettings, SirenSounds, Voice};

/// The longest a played result is waited for, in seconds. **Design default**: a library of short effects and a few music beds
/// is the case this example is for, and a longer track is a thing to start in a real player.
const LONGEST_WAIT_S: f64 = 20.0;

struct Options {
    library: PathBuf,
    description: String,
    text_model: Option<PathBuf>,
    vocab: Option<PathBuf>,
    audio_model: Option<PathBuf>,
    version: Option<String>,
    k: usize,
    pick: usize,
    bus: String,
    play: bool,
}

fn usage() -> String {
    format!(
        "usage: semantic <library-dir> \"<description>\" [--model clap-text.onnx --vocab tokenizer.json] [--audio-model \
         clap-audio.onnx]\n                [--model-version <v>] [--k {}] [--pick 1] [--bus Music] [--no-play]\n\nThe model files \
         are made once by `python3 tools/export_clap_onnx.py <dir>`; each flag also reads SIREN_MODEL, SIREN_VOCAB, \
         SIREN_AUDIO_MODEL and SIREN_MODEL_VERSION.",
        tools::DEFAULT_K
    )
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        library: PathBuf::new(),
        description: String::new(),
        text_model: env("SIREN_MODEL").map(PathBuf::from),
        vocab: env("SIREN_VOCAB").map(PathBuf::from),
        audio_model: env("SIREN_AUDIO_MODEL").map(PathBuf::from),
        version: env("SIREN_MODEL_VERSION"),
        k: tools::DEFAULT_K,
        pick: 1,
        bus: String::from("Music"),
        play: true,
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
            "--bus" => options.bus = value("a bus name")?,
            "--k" => {
                options.k = value("a number")?.parse().map_err(|_| String::from("--k is a whole number"))?
            }
            "--pick" => {
                options.pick =
                    value("a number")?.parse().map_err(|_| String::from("--pick is a whole number"))?
            }
            "--no-play" => options.play = false,
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            other => positional.push(other.to_owned()),
        }
    }
    let [library, description] = <[String; 2]>::try_from(positional).map_err(|got| {
        format!(
            "two arguments are required — a library directory and a description — and {} were given",
            got.len()
        )
    })?;
    options.library = PathBuf::from(library);
    options.description = description;
    tools::check_k(options.k).map_err(|e| e.to_string())?;
    if options.pick == 0 || options.pick > options.k {
        return Err(format!("--pick must be between 1 and --k ({}), and {} is not", options.k, options.pick));
    }
    Ok(options)
}

/// What the folder's index is, before any model is loaded — so a missing audio graph is reported **first**, not after a
/// 500 MB text graph has been loaded for nothing.
enum Existing {
    /// Nothing on disk yet.
    None,
    /// A file is there and cannot be used, with the loader's own reason (a stamp mismatch, a truncated write).
    Unusable(String),
    /// Usable, and the folder has since gained or lost files.
    Changed { new: usize, gone: usize },
    /// Usable and current: no audio graph needed.
    Fresh(Index),
}

fn existing(root: &Path, library: &Library, stamp: &ModelStamp) -> Existing {
    if !root.join(indexer::INDEX_FILE).exists() {
        return Existing::None;
    }
    match indexer::read(root, stamp) {
        Err(why) => Existing::Unusable(why),
        Ok(index) => {
            let new = library.assets.keys().filter(|h| !index.records.contains_key(h)).count();
            let gone = index.records.keys().filter(|h| !library.assets.contains_key(h)).count();
            if new == 0 && gone == 0 { Existing::Fresh(index) } else { Existing::Changed { new, gone } }
        }
    }
}

fn seconds(since: Instant) -> f64 {
    since.elapsed().as_secs_f64()
}

/// The first line of a message, cut to a screenful: the runtime's own errors run to several lines, and a library of a hundred
/// bad files must not be a hundred-line wall.
fn short(message: &str) -> String {
    let line = message.lines().next().unwrap_or("");
    match line.char_indices().nth(150) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_owned(),
    }
}

/// **The extractor's own mel filterbank** — 64 × 513 numbers, band-major — which is the *model's* data, not this crate's: it was
/// copied from `ClapFeatureExtractor.mel_filters` and is checked in for the tests. An example runs from a checkout, so it reads
/// it from there; a project using the library supplies the same numbers from its own copy of the extractor.
fn filterbank() -> Result<Filterbank, String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/clap-mel-filters.txt");
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read `{path}`: {e}"))?;
    let weights: Vec<f32> = text.lines().filter_map(|l| l.trim().parse().ok()).collect();
    Filterbank::from_weights(weights).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("semantic: {why}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };
    let stamp = match &options.version {
        Some(version) => ModelStamp::clap(version.as_str()),
        None => ModelStamp::measured_clap(),
    };
    // The description is checked **first**, with the other arguments: an empty one used to be found after the 501 MB text graph
    // had been loaded, which is the cost this pre-flight exists to avoid.
    let query = match Description::new(&options.description) {
        Ok(query) => query,
        Err(why) => {
            eprintln!("semantic: that description cannot be searched: {why}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };

    // 1. The folder, and what is already known about it — no model yet.
    let root = options.library.as_path();
    if !root.is_dir() {
        eprintln!("semantic: `{}` is not a directory", root.display());
        return ExitCode::from(2);
    }
    let library = library::scan(root);
    if library.assets.is_empty() {
        eprintln!(
            "semantic: no audio files in `{}` (WAV and Ogg Vorbis are what the decoder reads)",
            root.display()
        );
        return ExitCode::from(1);
    }
    let found = existing(root, &library, &stamp);
    let building = !matches!(found, Existing::Fresh(_));

    // 2. **What it needs, said before anything is loaded, written or played.**
    let mut missing: Vec<String> = Vec::new();
    let mut need = |flag: &str, env: &str, what: &str, path: &Option<PathBuf>| match path {
        None => missing.push(format!("  {flag:<14} {what} — not given (or set {env})")),
        Some(p) if !p.is_file() => {
            missing.push(format!("  {flag:<14} {what} — `{}` does not exist", p.display()))
        }
        Some(_) => {}
    };
    need("--model", "SIREN_MODEL", "the CLAP text graph, clap-text.onnx", &options.text_model);
    need("--vocab", "SIREN_VOCAB", "its tokenizer.json", &options.vocab);
    if building {
        need(
            "--audio-model",
            "SIREN_AUDIO_MODEL",
            "the CLAP audio graph, clap-audio.onnx (the folder has to be indexed)",
            &options.audio_model,
        );
    }
    if !missing.is_empty() {
        eprintln!("semantic: cannot run — {} file(s) missing:\n{}\n", missing.len(), missing.join("\n"));
        eprintln!(
            "The crate does not ship the model (CLAP, laion/clap-htsat-fused, MIT). Make the files once, from the weights in"
        );
        eprintln!("your Hugging Face cache or by download:\n\n    python3 tools/export_clap_onnx.py <dir>\n");
        eprintln!(
            "It writes clap-text.onnx, clap-audio.onnx, tokenizer.json and clap-export.txt. This build runs the model on the CPU"
        );
        eprintln!(
            "({}), so a machine with no GPU is fine; a machine with no model is not.",
            Device::available().describe()
        );
        return ExitCode::from(2);
    }
    let (Some(text_path), Some(vocab_path)) = (&options.text_model, &options.vocab) else {
        return ExitCode::from(2);
    };

    println!(
        "semantic: {} — {} assets in `{}`",
        Device::available().describe(),
        library.assets.len(),
        root.display()
    );
    println!(
        "          model version {}{}",
        stamp.version,
        if options.version.is_none() {
            "  (the one this crate measured; pass --model-version if your export differs)"
        } else {
            ""
        }
    );

    // 3. The index: reused when it is current, built with the audio tower when it is not.
    let index = match found {
        Existing::Fresh(index) => {
            println!(
                "index:    {} assets, read from `{}` — current, so no audio graph was loaded",
                index.len(),
                indexer::INDEX_FILE
            );
            index
        }
        other => {
            match &other {
                Existing::Unusable(why) => {
                    println!("index:    the file on disk cannot be used ({why}); building a new one")
                }
                Existing::Changed { new, gone } => {
                    println!("index:    the folder changed ({new} new, {gone} gone); rebuilding")
                }
                _ => println!("index:    none yet; building"),
            }
            let Some(audio_path) = &options.audio_model else { return ExitCode::from(2) };
            let started = Instant::now();
            let mut audio = match Model::load(&audio_path.to_string_lossy()) {
                Ok(model) => model,
                Err(why) => {
                    eprintln!("semantic: cannot load the audio graph: {why}");
                    return ExitCode::from(1);
                }
            };
            println!("          audio graph loaded in {:.1} s", seconds(started));
            let filterbank = match filterbank() {
                Ok(filterbank) => filterbank,
                Err(why) => {
                    eprintln!("semantic: {why}");
                    return ExitCode::from(1);
                }
            };
            let started = Instant::now();
            let built = match indexer::build_from_model(root, &mut audio, &filterbank, stamp.clone()) {
                Ok(built) => built,
                Err(why) => {
                    eprintln!("semantic: cannot build the index: {why}");
                    return ExitCode::from(1);
                }
            };
            let took = seconds(started);
            println!(
                "          {} in {took:.1} s ({:.1} clips/s)",
                short(&built.describe()),
                built.indexed() as f64 / took.max(1e-9)
            );
            for failure in built.failed.iter().take(3) {
                println!("          could not index `{}`: {}", failure.path.display(), short(&failure.why));
            }
            if built.failed.len() > 3 {
                println!("          … and {} more", built.failed.len() - 3);
            }
            // **An index that holds nothing is not written**: it would be a file on disk claiming to be this folder's index, and
            // the first real run of this example wrote exactly that — 0 indexed, 14 failed — and went on to answer *"nothing
            // matched"*, which blamed the description for a graph that could not run.
            if built.indexed() == 0 {
                eprintln!(
                    "semantic: none of the {} files could be indexed, so no index was written",
                    library.assets.len()
                );
                return ExitCode::from(1);
            }
            match indexer::write(root, &built.index) {
                Ok(path) => println!("          written to `{}`", path.display()),
                Err(why) => eprintln!("semantic: the index was built but not saved: {why}"),
            }
            built.index
        }
    };

    // 4. Ask: the text tower, then the same ranked search `search_music` answers over MCP.
    let started = Instant::now();
    let vocabulary = match std::fs::read_to_string(vocab_path)
        .map_err(|e| e.to_string())
        .and_then(|t| Vocabulary::from_json(&t).map_err(|e| e.to_string()))
    {
        Ok(vocabulary) => vocabulary,
        Err(why) => {
            eprintln!("semantic: cannot read the vocabulary `{}`: {why}", vocab_path.display());
            return ExitCode::from(1);
        }
    };
    let mut text = match Model::load(&text_path.to_string_lossy()) {
        Ok(model) => model,
        Err(why) => {
            eprintln!("semantic: cannot load the text graph: {why}");
            return ExitCode::from(1);
        }
    };
    println!("text:     graph loaded in {:.1} s", seconds(started));
    let started = Instant::now();
    let embedding = match Embedder::new(&vocabulary, &mut text).embed(&query) {
        Ok(embedding) => embedding,
        Err(why) => {
            eprintln!("semantic: cannot embed the description: {why}");
            return ExitCode::from(1);
        }
    };
    let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
    let features = tools::features_by_hash(&index);
    let toolset = Toolset::default();
    // **No anchor**: a hash the index does not hold, which the ranking treats as having no acoustic features — a ranking by the
    // description alone. (`search_music` over MCP does the same when no `anchor` is passed.)
    let unanchored = (0u64..).find(|h| !index.records.contains_key(h)).unwrap_or(0);
    let request = Ranked {
        embedding: &embedding,
        anchor: unanchored,
        features: &features,
        k: options.k,
        floor: Floor::default(),
        weights: &toolset.music,
    };
    let started = Instant::now();
    let answer = match tools::search_music(&index, &stamp, &query, &request) {
        Ok(answer) => answer,
        Err(why) => {
            eprintln!("semantic: the search was refused: {why}");
            return ExitCode::from(1);
        }
    };
    println!(
        "          encoded in {encode_ms:.1} ms, ranked {} assets in {:.2} ms\n",
        index.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    println!("\"{}\"  — top {}", options.description, options.k);
    if answer.results.is_empty() {
        println!("  nothing in this library matched ({})", answer.tally.describe());
        return ExitCode::from(1);
    }
    for (i, result) in answer.results.iter().enumerate() {
        let name = name_of(&library, result);
        println!("  {}. {name}   score {:.3}   similarity {:.3}", i + 1, result.score, result.similarity);
        println!("     {}", result.explain());
        println!("     {}", result.provenance);
    }
    println!("  ({})", answer.tally.describe());
    // A score is a cosine on CLAP's own scale, not a percentage, and a first-time reader takes 0.2 for a miss. The guide has the
    // measured bands; the example points there rather than copying a number that could go stale.
    println!(
        "  scores are CLAP cosines, not percentages — what a good one looks like is in docs/guide/semantic.md, \"The floor\""
    );

    // 5. Hear it — through the crate's own play path.
    let Some(chosen) = answer.results.get(options.pick - 1) else {
        println!("\nthere is no result {} to play", options.pick);
        return ExitCode::from(1);
    };
    if !options.play {
        println!("\n--no-play: nothing was played");
        return ExitCode::SUCCESS;
    }
    play(root, &library, &index, chosen, &options.bus)
}

fn name_of(library: &Library, found: &Found) -> String {
    library
        .assets
        .get(&found.hash)
        .and_then(|a| a.paths.first())
        .map_or_else(|| format!("{:016x}", found.hash), |p| p.display().to_string())
}

/// How long to wait for a file to sound: its length when the header says it, read as 16-bit PCM, else a few seconds.
fn wait_for(library: &Library, hash: u64) -> f64 {
    let estimate = library.assets.get(&hash).and_then(|a| {
        let (rate, channels) = (f64::from(a.sample_rate?), f64::from(a.channels?));
        Some(a.bytes as f64 / (rate * channels * 2.0))
    });
    (estimate.unwrap_or(5.0) + 0.75).clamp(1.0, LONGEST_WAIT_S)
}

#[derive(Resource)]
struct Wait {
    seconds: f64,
    started: f64,
    heard: bool,
    warned: bool,
}

fn play(root: &Path, library: &Library, index: &Index, chosen: &Found, bus: &str) -> ExitCode {
    let Some(relative) = library.assets.get(&chosen.hash).and_then(|a| a.paths.first()) else {
        eprintln!("semantic: the library has no path for the chosen asset");
        return ExitCode::from(1);
    };
    let sound = relative.to_string_lossy().into_owned();
    // **`tools::audition` is the one place an audition's cue is made**: derived, one sound slot, one instance, not spatial.
    let audition = match tools::audition(index, chosen.hash, library, &sound, bus) {
        Ok(audition) => audition,
        Err(why) => {
            eprintln!("semantic: cannot audition: {why}");
            return ExitCode::from(1);
        }
    };
    let bytes = match std::fs::read(root.join(relative)) {
        Ok(bytes) => bytes,
        Err(why) => {
            eprintln!("semantic: cannot read `{}`: {why}", relative.display());
            return ExitCode::from(1);
        }
    };
    println!("\nplaying the chosen result on the {bus} bus …");

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(10))),
        AssetPlugin::default(),
        AudioPlugin::default(),
        SirenPlugin,
    ))
    .insert_resource(SirenSettings::default())
    .insert_resource(CueTable::new(vec![audition.cue.clone()]))
    .insert_resource(Wait {
        seconds: wait_for(library, chosen.hash),
        started: -1.0,
        heard: false,
        warned: false,
    })
    .add_systems(
        Startup,
        move |mut sounds: ResMut<SirenSounds>,
              mut assets: ResMut<Assets<AudioSource>>,
              mut plays: MessageWriter<Play>| {
            // The bytes are handed over as they are: Bevy's own decoder reads the format, so what plays is the file.
            sounds.insert(sound.clone(), assets.add(AudioSource { bytes: bytes.clone().into() }));
            plays.write(audition.play.clone());
        },
    )
    .add_systems(Update, report);
    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(code) => ExitCode::from(code.get()),
    }
}

/// **The answer, as the crate gives it**: exactly one `Played` or `Refused`, then the end of the sound.
fn report(
    mut played: MessageReader<Played>,
    mut refused: MessageReader<Refused>,
    voices: Query<&Voice>,
    sinks: Query<(), With<AudioSink>>,
    time: Res<Time<Real>>,
    mut wait: ResMut<Wait>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs_f64();
    for answer in played.read() {
        println!(
            "Played    {} on the {} bus at {:+.1} dB, variant `{}`",
            answer.cue, answer.bus, answer.gain_db, answer.sound
        );
        wait.started = now;
        wait.heard = true;
    }
    if let Some(answer) = refused.read().next() {
        println!("Refused   {}: {}", answer.cue, answer.reason);
        exit.write(AppExit::from_code(1));
        return;
    }
    if !wait.heard {
        return;
    }
    let elapsed = now - wait.started;
    // **Without an audio device the voice exists and never gets a sink** (this crate's own trap), so say so rather than
    // wait out the clock in silence.
    if !wait.warned && elapsed > 0.6 && voices.iter().next().is_some() && sinks.iter().next().is_none() {
        println!("          (no audio device opened: the play was answered, but nothing could sound)");
        wait.warned = true;
    }
    let ended = voices.iter().next().is_none() && elapsed > 0.3;
    if ended || elapsed > wait.seconds {
        println!("done");
        exit.write(AppExit::Success);
    }
}
