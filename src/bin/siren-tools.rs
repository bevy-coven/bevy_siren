//! **`siren-tools`**: ADR 0006's tool server — an MCP host over stdio for choosing audio from an indexed library.
//!
//! ```sh
//! cargo run --release --features semantic --bin siren-tools -- <library-dir> [--model <file.onnx> --vocab <tokenizer.json>]
//! ```
//!
//! It reads **one JSON-RPC message per line on stdin** and writes **one per line on stdout**; everything else — the start-up
//! summary, a refusal to start — goes to **stderr**, because the transport says stdout carries nothing that is not a message.
//! Closing stdin ends it with status 0. All of the protocol is `bevy_siren::semantic::server`; this file is the loop around
//! it, so the rules are tested without a pipe and this file is tested as a process.
//!
//! `<library-dir>` is a directory holding the index file the indexer wrote (`indexer::INDEX_FILE`). With `--model` **and** `--vocab`,
//! `search_music` embeds descriptions with that graph; without them it answers `unavailable` and says what to add, and the
//! tools that need only the index (`set_anchor`, `find_compatible_*`, `explain_match`) work either way.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use bevy_siren::semantic::ask::Embedder;
use bevy_siren::semantic::index::ModelStamp;
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::model::Model;
use bevy_siren::semantic::query::Query;
use bevy_siren::semantic::server::{Embed, Host, MAX_LINE_BYTES, read_message};
use bevy_siren::semantic::tokenizer::Vocabulary;

fn usage() -> String {
    format!(
        "usage: siren-tools <library-dir> [--model <file.onnx> --vocab <tokenizer.json>]\n\n  <library-dir>  the directory \
         holding the `{}` file the indexer wrote\n  --model        the CLAP text graph, so `search_music` can embed a \
         description\n  --vocab        that model's tokenizer.json (--model and --vocab go together)",
        indexer::INDEX_FILE
    )
}

/// A vocabulary and the model it belongs to, owned, so the host can embed for as long as it runs.
struct Clap {
    vocabulary: Vocabulary,
    model: Model,
}

impl Embed for Clap {
    fn embed(&mut self, query: &Query) -> Result<Vec<f32>, String> {
        Embedder::new(&self.vocabulary, &mut self.model).embed(query).map_err(|e| e.to_string())
    }
}

struct Options {
    library: PathBuf,
    model: Option<(String, PathBuf)>,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let (mut library, mut model, mut vocab) = (None, None, None);
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--model" => model = Some(args.next().ok_or("--model needs a file")?),
            "--vocab" => vocab = Some(args.next().ok_or("--vocab needs a file")?),
            flag if flag.starts_with('-') => return Err(format!("unknown option `{flag}`")),
            dir if library.is_none() => library = Some(PathBuf::from(dir)),
            extra => return Err(format!("unexpected argument `{extra}`: one library directory only")),
        }
    }
    let library = library.ok_or("a library directory is required")?;
    let model = match (model, vocab) {
        (Some(model), Some(vocab)) => Some((model, PathBuf::from(vocab))),
        (None, None) => None,
        _ => {
            return Err(String::from(
                "--model and --vocab go together: a graph without its tokenizer cannot embed",
            ));
        }
    };
    Ok(Options { library, model })
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("siren-tools: {why}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };

    // **The index is checked against the one model this crate has evidence for.** A mismatch is refused by the loader, with
    // its own sentence, rather than served: ranking across two vector spaces looks like a working feature doing a bad job.
    let stamp = ModelStamp::measured_clap();
    let index = match indexer::read(&options.library, &stamp) {
        Ok(index) => index,
        Err(why) => {
            eprintln!("siren-tools: cannot use the index in `{}`: {why}", options.library.display());
            return ExitCode::from(1);
        }
    };

    let embedder: Option<Box<dyn Embed>> = match &options.model {
        None => None,
        Some((model_path, vocab_path)) => {
            let model = match Model::load(model_path) {
                Ok(model) => model,
                Err(why) => {
                    eprintln!("siren-tools: cannot load the model: {why}");
                    return ExitCode::from(1);
                }
            };
            let vocabulary = match std::fs::read_to_string(vocab_path)
                .map_err(|e| format!("cannot read `{}`: {e}", vocab_path.display()))
                .and_then(|text| Vocabulary::from_json(&text).map_err(|e| e.to_string()))
            {
                Ok(vocabulary) => vocabulary,
                Err(why) => {
                    eprintln!("siren-tools: cannot load the vocabulary: {why}");
                    return ExitCode::from(1);
                }
            };
            Some(Box::new(Clap { vocabulary, model }))
        }
    };

    let mut host = Host::new(index, stamp, embedder);
    eprintln!(
        "siren-tools: {} assets indexed from `{}`; search_music is {}; reading JSON-RPC from stdin",
        host.indexed(),
        options.library.display(),
        if host.can_embed() { "on" } else { "off (no --model/--vocab)" },
    );

    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let line = match read_message(&mut input, MAX_LINE_BYTES) {
            Ok(Some(line)) => line,
            // End of input is how a client closes the session.
            Ok(None) => return ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("siren-tools: cannot read stdin: {why}");
                return ExitCode::from(1);
            }
        };
        let Some(reply) = host.handle_line(line) else { continue };
        // A client that closed its end of the pipe is a normal way to finish, not an error to report.
        if writeln!(output, "{reply}").and_then(|()| output.flush()).is_err() {
            return ExitCode::SUCCESS;
        }
    }
}
