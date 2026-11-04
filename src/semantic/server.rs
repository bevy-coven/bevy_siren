//! **The tool host**: ADR 0006's `siren-tools`, written as a library so a test can drive it with no process.
//!
//! One [`Host`] is one MCP session over **newline-delimited JSON-RPC 2.0**, the stdio transport the specification says
//! clients SHOULD support. [`Host::handle`] takes one line and returns at most one line; the binary in
//! `src/bin/siren-tools.rs` is a loop around it and nothing else, so every rule below is testable without a pipe.
//!
//! # What it speaks
//!
//! Five methods, and nothing else: `initialize`, `ping`, `tools/list`, `tools/call`, and the client's
//! `notifications/initialized` (a notification, so it gets no reply). The rules come from the specification's own pages
//! (`basic/transports`, `basic/lifecycle`, `server/tools`, revision `2025-06-18`):
//!
//! - a message is **one line with no embedded newline**, and the server writes **nothing** to stdout that is not a message —
//!   [`Host::handle`] returns compact JSON, which cannot contain a raw newline;
//! - `initialize` is answered with the **requested** protocol version if this server supports it, else its own latest;
//! - a notification is **never answered**, and never executes a method, whatever it names;
//! - an unknown *method* is `-32601`, an unknown *tool* is `-32602` (the specification's own example), and a message that is
//!   not JSON, or not an object, is `-32700` or `-32600` with an `id` of `null`.
//!
//! # One table drives the listing, the validation and the schema
//!
//! [`SPECS`] has one row per tool. `tools/list` renders each row to an `inputSchema`, and `tools/call` validates the
//! arguments against **the same row** before anything runs — so the schema a client reads and the check its call meets cannot
//! disagree, because there is one of them. A tool in `SPECS` with no handler answers `internal` rather than silently
//! succeeding, and `tests/semantic_server.rs` calls every row to prove none does.
//!
//! # Errors: protocol errors for the protocol, tool results for the model
//!
//! The specification names two mechanisms: JSON-RPC errors for protocol problems, and a result with `isError: true` for a
//! tool that ran and failed. **Every failure of a tool — including a bad argument — is the second.** A model reads tool
//! results; a JSON-RPC error is addressed to the client program, which may never show it. ADR 0007's case is that *"an
//! agent that cannot parse an error cannot recover"*, so `k: 0` is a result carrying
//! `{"error": {"code": "bad_argument", "message": …, "retryable": true}}`, and the model can fix the call.
//!
//! # State, scope and cost (`#28`'s pitfall)
//!
//! **The anchor is the host's only state**, and it belongs to this host: one connection, one [`Host`], lost when it drops.
//! Nothing is global, so two clients cannot interleave (`#42`'s pitfall is solved by having nothing to share).
//!
//! **The index is read once at start-up and held in memory; a query is a scan, not a load.** A long-lived process is the
//! expectation — the stdio transport launches one server per client session and closes it by closing stdin. Start-up is the
//! cost of reading the index file (`indexer::INDEX_FILE`) and, if a model was given, loading the ONNX graph; both are measured in
//! `docs/loop/runs/2026-11-03-109/perf.md`.
//!
//! # What this is not
//!
//! It plays nothing. `audition` needs a running app (ADR 0006: *"the tool server NEVER owns playback"*), so it is listed —
//! an agent should learn it exists and why it cannot run — and answers `unavailable`.

use std::collections::BTreeMap;
use std::io::{self, BufRead};

use serde_json::{Map, Number, Value, json};

use crate::model::Breakdown;

use super::filter::Features;
use super::index::{Index, ModelStamp};
use super::key::Compatibility;
use super::provenance::{Provenance, Source};
use super::query::Query;
use super::rank::Floor;
use super::search::Ranked;
use super::tools::{self, Code, Found, Idempotency, MAX_K, Session, ToolProblem, Toolset, check_k};
use super::weights::Weights;

/// The protocol revisions this host answers, **latest first**. The three share the shapes of every method here; the
/// lifecycle rule is *"if the server supports the requested version it MUST respond with the same one"*.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// The longest line the host reads, in bytes. **Design default `1 MiB`**: the largest legitimate message here is a
/// `tools/call` carrying a description of a few hundred bytes, so this is three orders of magnitude of headroom and a
/// hard stop on a client that never sends a newline.
pub const MAX_LINE_BYTES: usize = 1 << 20;

/// `find_compatible_sfx`'s default `k`. ADR 0007: *"an effect library's useful neighbourhood is larger"* than music's
/// [`tools::DEFAULT_K`] — *"twenty footsteps that belong together are a set, not a shortlist"*.
pub const SFX_DEFAULT_K: usize = 6;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// What `initialize` tells a client about using the tools, in the specification's optional `instructions` field.
const INSTRUCTIONS: &str = "Tools for choosing audio from an indexed library. Describe what you want with `search_music` \
    (it needs a model), pick one result, `set_anchor` it, then ask `find_compatible_music` or `find_compatible_sfx` for \
    what belongs beside it and `explain_match` for why something ranked where it did. An `asset_id` is the 16-character \
    content hash a result returned, never a path. The anchor belongs to this connection and is lost when it closes. `k` is \
    a bound from 1 to 100, not a page: ask for a larger `k` to see more.";

/// **Embeds a description**, so `search_music` can rank without this module owning a model.
///
/// A trait rather than a [`super::model::Model`] because the host's tests need to supply a deterministic embedding, and
/// because a host with no model at all is a legitimate configuration: `search_music` then answers `unavailable`.
pub trait Embed {
    /// The embedding of `query`, on the index's own scale, or why it could not be made.
    fn embed(&mut self, query: &Query) -> Result<Vec<f32>, String>;
}

/// **What a parameter is**, which decides both its schema and how a call's value is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A non-empty string.
    Text,
    /// A content hash, as **16 lowercase hex characters** — ADR 0007's `asset_id`. Never a path.
    AssetId,
    /// The `k` of a ranking: an integer from 1 to [`MAX_K`]. **A bound, not a page** (ADR 0007).
    K {
        /// What an omitted `k` means for this tool.
        default: usize,
    },
    /// One of a fixed set of words. An omitted argument means the **first**.
    OneOf(&'static [&'static str]),
}

/// One parameter of one tool.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    /// The name a call uses.
    pub name: &'static str,
    /// What it is.
    pub kind: Kind,
    /// Whether a call must give it.
    pub required: bool,
    /// What it means, in the sentence a model reads beside the schema.
    pub description: &'static str,
}

/// One tool: the row every part of the host reads.
#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    /// The name a client calls. Must be a row of [`tools::TOOLS`], which supplies the idempotency.
    pub name: &'static str,
    /// What it does and when to use it. The idempotency sentence is appended at listing time.
    pub description: &'static str,
    /// Whether it only reads. `false` for a tool that writes the session's anchor or starts a voice.
    pub read_only: bool,
    /// Its parameters, in the order a schema lists them.
    pub params: &'static [Param],
}

const ASSET_ID: &str = "The asset: 16 lowercase hex characters, exactly as an earlier result returned it.";

/// **The six tools of ADR 0007.** The one table the listing, the schemas and the validation all read.
pub const SPECS: [ToolSpec; 6] = [
    ToolSpec {
        name: "search_music",
        description: "Find audio in the library by describing it in words, best match first. Returns at most `k` \
            results, each with its score, why it scored that way, and where the asset came from. It does not use the \
            session's anchor: pass `anchor` to judge every result against that asset's loudness, tempo and key.",
        read_only: true,
        params: &[
            Param {
                name: "description",
                kind: Kind::Text,
                required: true,
                description: "What you want to hear, in words: for example `tense, sparse strings, underwater ruins`.",
            },
            Param {
                name: "k",
                kind: Kind::K { default: tools::DEFAULT_K },
                required: false,
                description: "How many results at most. A bound, not a page: ask for a larger `k` to see more.",
            },
            Param {
                name: "anchor",
                kind: Kind::AssetId,
                required: false,
                description: "An asset to judge results against, as 16 lowercase hex characters. Omit to rank by the \
                    description alone.",
            },
        ],
    },
    ToolSpec {
        name: "set_anchor",
        description: "Choose the asset the next questions are about, and learn what the library knows about it: \
            loudness, spectrum, tempo, key and where it came from. `find_compatible_music` and `find_compatible_sfx` use \
            it when you pass no `anchor`. It belongs to this connection and is lost when the connection closes.",
        read_only: false,
        params: &[Param { name: "asset_id", kind: Kind::AssetId, required: true, description: ASSET_ID }],
    },
    ToolSpec {
        name: "find_compatible_music",
        description: "Find music that fits the anchor, best first: similar in sound and compatible in loudness, tempo and \
            key. It reports what it excluded and why, so a short list is explained. Uses the anchor `set_anchor` chose \
            unless you pass one.",
        read_only: true,
        params: &[
            Param {
                name: "anchor",
                kind: Kind::AssetId,
                required: false,
                description: "The asset to fit, as 16 lowercase hex characters. Omit to use the anchor `set_anchor` chose.",
            },
            Param {
                name: "k",
                kind: Kind::K { default: tools::DEFAULT_K },
                required: false,
                description: "How many results at most. A bound, not a page: ask for a larger `k` to see more.",
            },
        ],
    },
    ToolSpec {
        name: "find_compatible_sfx",
        description: "Find sound effects that belong in the same world as the anchor, best first. Near-identical variants \
            of one sound are reported, not returned a dozen times. Uses the anchor `set_anchor` chose unless you pass one.",
        read_only: true,
        params: &[
            Param {
                name: "anchor",
                kind: Kind::AssetId,
                required: false,
                description: "The asset to fit, as 16 lowercase hex characters. Omit to use the anchor `set_anchor` chose.",
            },
            Param {
                name: "category",
                kind: Kind::Text,
                required: false,
                description: "Keep only assets whose recorded source contains this text, case-insensitively: for example \
                    `footsteps`.",
            },
            Param {
                name: "k",
                kind: Kind::K { default: SFX_DEFAULT_K },
                required: false,
                description: "How many results at most. A bound, not a page: ask for a larger `k` to see more.",
            },
        ],
    },
    ToolSpec {
        name: "explain_match",
        description: "Explain why `asset_id` scores what it does against `anchor`: every signal's raw value, weight and \
            contribution, and a sentence naming what carried it and what counted against it. Use it when a ranking looks \
            wrong, and pass the `weighting` of the search that ranked the pair.",
        read_only: true,
        params: &[
            Param { name: "asset_id", kind: Kind::AssetId, required: true, description: ASSET_ID },
            Param {
                name: "anchor",
                kind: Kind::AssetId,
                required: true,
                description: "The asset it is compared against, as 16 lowercase hex characters.",
            },
            Param {
                name: "weighting",
                kind: Kind::OneOf(&["music", "effects"]),
                required: false,
                description: "Which weighting scored the pair: `music` for a result of `search_music` or \
                    `find_compatible_music`, `effects` for one of `find_compatible_sfx`. Under the wrong one the weights \
                    do not add up to the score the search returned.",
            },
        ],
    },
    ToolSpec {
        name: "audition",
        description: "Play an asset through the game's own audio so a person can hear it. It needs a running app, and \
            this server has none, so it answers `unavailable`.",
        read_only: false,
        params: &[
            Param { name: "asset_id", kind: Kind::AssetId, required: true, description: ASSET_ID },
            Param {
                name: "bus",
                kind: Kind::Text,
                required: false,
                description: "The bus to play on. `Effects` when omitted.",
            },
        ],
    },
];

impl Param {
    /// This parameter as a JSON Schema property.
    fn schema(&self) -> Value {
        match self.kind {
            Kind::Text => json!({ "type": "string", "minLength": 1, "description": self.description }),
            Kind::AssetId => {
                json!({ "type": "string", "pattern": "^[0-9a-f]{16}$", "description": self.description })
            }
            Kind::OneOf(options) => json!({
                "type": "string",
                "enum": options,
                "default": options.first(),
                "description": self.description,
            }),
            Kind::K { default } => json!({
                "type": "integer",
                "minimum": 1,
                "maximum": MAX_K,
                "default": default,
                "description": self.description,
            }),
        }
    }
}

/// A call's arguments after validation: typed, with defaults applied.
#[derive(Debug, Default)]
struct Args {
    text: BTreeMap<&'static str, String>,
    id: BTreeMap<&'static str, u64>,
    k: Option<usize>,
    choice: BTreeMap<&'static str, &'static str>,
}

impl ToolSpec {
    /// This tool's `inputSchema`.
    fn input_schema(&self) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for param in self.params {
            properties.insert(param.name.to_owned(), param.schema());
            if param.required {
                required.push(param.name);
            }
        }
        json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
    }

    /// **Check a call's arguments against this row**, before any work happens. Every problem is reported in one message,
    /// so an agent with two mistakes fixes both in one retry.
    fn check(&self, given: &Map<String, Value>) -> Result<Args, String> {
        let mut problems = Vec::new();
        let mut args = Args::default();
        for key in given.keys() {
            if !self.params.iter().any(|p| p.name == key) {
                let takes: Vec<String> = self.params.iter().map(|p| format!("`{}`", p.name)).collect();
                problems.push(format!(
                    "`{key}` is not an argument of `{}`, which takes {}",
                    self.name,
                    takes.join(", ")
                ));
            }
        }
        for param in self.params {
            let value = given.get(param.name).filter(|v| !v.is_null());
            let Some(value) = value else {
                match param.kind {
                    _ if param.required => problems.push(format!("`{}` is required", param.name)),
                    Kind::K { default } => args.k = Some(default),
                    Kind::OneOf(options) => {
                        if let Some(first) = options.first() {
                            args.choice.insert(param.name, first);
                        }
                    }
                    _ => {}
                }
                continue;
            };
            match param.kind {
                Kind::Text => match value.as_str().filter(|s| !s.trim().is_empty()) {
                    Some(text) => {
                        args.text.insert(param.name, text.to_owned());
                    }
                    None => problems.push(format!("`{}` must be a non-empty string", param.name)),
                },
                Kind::AssetId => match value.as_str().and_then(parse_asset_id) {
                    Some(hash) => {
                        args.id.insert(param.name, hash);
                    }
                    None => problems.push(format!(
                        "`{}` must be an asset id: 16 lowercase hex characters, exactly as an earlier result returned it",
                        param.name
                    )),
                },
                Kind::K { .. } => match as_whole(value).and_then(|n| usize::try_from(n).ok()) {
                    // **`check_k` is the one place that decides the range**, so its sentence is the one a library caller
                    // reads too.
                    Some(k) => match check_k(k) {
                        Ok(k) => args.k = Some(k),
                        Err(problem) => problems.push(problem.to_string()),
                    },
                    None => problems.push(format!("`{}` must be an integer from 1 to {MAX_K}", param.name)),
                },
                Kind::OneOf(options) => match value.as_str().and_then(|s| options.iter().copied().find(|o| *o == s)) {
                    Some(choice) => {
                        args.choice.insert(param.name, choice);
                    }
                    None => {
                        let allowed: Vec<String> = options.iter().map(|o| format!("`{o}`")).collect();
                        problems.push(format!("`{}` must be one of {}", param.name, allowed.join(", ")));
                    }
                },
            }
        }
        if problems.is_empty() { Ok(args) } else { Err(problems.join("; ")) }
    }
}

/// An asset id as the hash it names: exactly 16 lowercase hex characters, nothing else.
fn parse_asset_id(text: &str) -> Option<u64> {
    if text.len() != 16 || !text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return None;
    }
    u64::from_str_radix(text, 16).ok()
}

/// **A JSON number that is a whole number**, as the schema's `"type": "integer"` means it: JSON Schema (2020-12) counts `3.0`
/// as an integer, and a model that writes `k: 3.0` has made no mistake the schema forbids.
fn as_whole(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        // 1e15 is far above `MAX_K` and well inside the range where an `f64` holds every integer exactly.
        value.as_f64().filter(|f| f.fract() == 0.0 && (0.0..1e15).contains(f)).map(|f| f as u64)
    })
}

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// **A float as the shortest decimal that reads back as the same `f32`**, so `0.683` is `0.683` on the wire and not the
/// `0.6830000281333923` that widening it to `f64` would print. `null` for a value JSON cannot carry.
fn num(value: f32) -> Value {
    if !value.is_finite() {
        return Value::Null;
    }
    value.to_string().parse::<f64>().ok().and_then(Number::from_f64).map_or(Value::Null, Value::Number)
}

fn optional(value: Option<f32>) -> Value {
    value.map_or(Value::Null, num)
}

fn breakdown_json(breakdown: &Breakdown) -> Value {
    let parts: Vec<Value> = breakdown
        .parts
        .iter()
        .map(|p| {
            json!({
                "signal": p.signal.name(),
                // **A missing signal is `null`, not `0`**: a signal with nothing to say and one that was consulted and
                // disagreed are different facts (ADR 0007).
                "raw": if p.missing { Value::Null } else { num(p.raw) },
                "weight": num(p.weight),
                "contribution": num(p.contribution),
                "missing": p.missing,
            })
        })
        .collect();
    json!({ "parts": parts })
}

fn provenance_json(provenance: &Provenance) -> Value {
    let Provenance { licence, source, author } = provenance;
    let source = source.as_ref().map(|s| match s {
        Source::Url(url) => format!("url:{url}"),
        Source::Pack(pack) => format!("pack:{pack}"),
        Source::Unknown => String::from("unknown"),
    });
    json!({ "licence": licence, "source": source, "author": author })
}

fn found_json(found: &Found) -> Value {
    json!({
        "asset_id": hex(found.hash),
        "score": num(found.score),
        "similarity": num(found.similarity),
        "breakdown": breakdown_json(&found.breakdown),
        "explain": found.explain(),
        "provenance": provenance_json(&found.provenance),
    })
}

/// **Every field of the weights reaches the wire**, because the destructuring is exhaustive: a field added to
/// [`Weights`] is a compile error here, not a silent omission from every answer that is supposed to be reproducible.
fn weights_json(weights: &Weights) -> Value {
    let Weights {
        semantic,
        acoustic,
        tempo,
        key,
        loudness_tolerance_db,
        noise_floor_tolerance_db,
        tempo_tolerance_bpm,
        compatibility,
        drop_fraction,
        min_similarity,
    } = *weights;
    // The same rule one level down: `Compatibility` is destructured exhaustively too.
    let Compatibility { clash_penalty, parallel_penalty } = compatibility;
    json!({
        "semantic": num(semantic),
        "acoustic": num(acoustic),
        "tempo": num(tempo),
        "key": num(key),
        "loudness_tolerance_db": num(loudness_tolerance_db),
        "noise_floor_tolerance_db": num(noise_floor_tolerance_db),
        "tempo_tolerance_bpm": num(tempo_tolerance_bpm),
        "compatibility": { "clash_penalty": num(clash_penalty), "parallel_penalty": num(parallel_penalty) },
        "drop_fraction": num(drop_fraction),
        "min_similarity": num(min_similarity),
    })
}

/// **What the index knows about an asset**, section by section. A section that was never measured is `null`; one that was
/// measured and found nothing (a clip that is not rhythmic) is an object with a `null` inside — different facts.
fn features_json(features: &Features) -> Value {
    let Features { loudness, spectral, tempo, key } = *features;
    json!({
        "loudness": loudness.map(|l| json!({
            "integrated_lufs": optional(l.integrated),
            "true_peak_dbtp": num(l.true_peak),
            "sample_peak_dbfs": num(l.sample_peak),
        })),
        "spectral": spectral.map(|s| json!({
            "centroid_hz": optional(s.centroid_hz),
            "rolloff_hz": optional(s.rolloff_hz),
            "noise_floor_db": num(s.noise_floor_db),
        })),
        "tempo": tempo.map(|t| json!({ "bpm": optional(t.bpm), "confidence": num(t.confidence) })),
        "key": key.map(|k| json!({ "name": k.key.name(), "confidence": num(k.confidence) })),
    })
}

fn answer_json(answer: &tools::Answer, summary: String) -> Value {
    let variants: Vec<Value> = answer
        .tally
        .variants
        .iter()
        .map(|(dropped, kept)| json!({ "asset_id": hex(*dropped), "resembles": hex(*kept) }))
        .collect();
    json!({
        "results": answer.results.iter().map(found_json).collect::<Vec<_>>(),
        "summary": summary,
        "tally": {
            "returned": answer.tally.returned,
            "excluded": answer.tally.excluded,
            "below_floor": answer.tally.below_floor,
            "variants": variants,
            "wrong_category": answer.tally.wrong_category,
        },
        "weights": weights_json(&answer.weights),
    })
}

/// A tool that could not answer, as ADR 0007's one error shape.
#[derive(Debug, Clone, PartialEq)]
struct Failure {
    code: Code,
    message: String,
}

impl Failure {
    fn new(code: Code, message: impl Into<String>) -> Failure {
        Failure { code, message: message.into() }
    }

    fn internal(what: impl Into<String>) -> Failure {
        Failure::new(Code::Internal, what)
    }
}

impl From<ToolProblem> for Failure {
    fn from(problem: ToolProblem) -> Failure {
        Failure::new(problem.code(), problem.to_string())
    }
}

/// A tool's result, as the specification shapes it: the structured object, **and** its serialisation in a text block, which
/// the specification says a tool returning structured content SHOULD do for older clients.
fn tool_result(structured: Value, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": structured.to_string() }],
        "structuredContent": structured,
        "isError": is_error,
    })
}

fn failure_result(failure: &Failure) -> Value {
    tool_result(
        json!({ "error": {
            "code": failure.code.as_str(),
            "message": failure.message,
            "retryable": failure.code.retryable(),
        } }),
        true,
    )
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

/// A protocol-level failure: a JSON-RPC code and its message.
type Protocol = (i64, String);

/// **One line from a message stream**, read with a ceiling so a client that never sends a newline cannot exhaust memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A complete line, without its terminator.
    Message(String),
    /// A line longer than the ceiling. Its bytes were consumed and discarded, so the next read starts on the next line.
    TooLong,
    /// A line that is not UTF-8, which JSON-RPC over MCP requires.
    NotUtf8,
}

/// **Read the next non-blank line**, at most `max` bytes of it kept. `Ok(None)` is end of input.
///
/// The framing is the transport's, so it lives beside the host: a line over the ceiling is **drained through its newline**
/// rather than left half-read, which is what keeps the stream aligned for the next message.
pub fn read_message<R: BufRead>(reader: &mut R, max: usize) -> io::Result<Option<Line>> {
    loop {
        let mut bytes: Vec<u8> = Vec::new();
        let mut too_long = false;
        let mut saw_any = false;
        loop {
            let available = match reader.fill_buf() {
                Ok(available) => available,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            if available.is_empty() {
                break;
            }
            saw_any = true;
            let newline = available.iter().position(|b| *b == b'\n');
            let chunk = &available[..newline.unwrap_or(available.len())];
            let used = chunk.len() + usize::from(newline.is_some());
            if !too_long {
                if bytes.len() + chunk.len() > max {
                    too_long = true;
                    bytes = Vec::new();
                } else {
                    bytes.extend_from_slice(chunk);
                }
            }
            reader.consume(used);
            if newline.is_some() {
                break;
            }
        }
        if !saw_any {
            return Ok(None);
        }
        if too_long {
            return Ok(Some(Line::TooLong));
        }
        // A `\r\n` terminator is a line ending, not part of the message.
        while bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        return Ok(Some(match String::from_utf8(bytes) {
            Ok(text) => Line::Message(text),
            Err(_) => Line::NotUtf8,
        }));
    }
}

/// **One MCP session**: an index, an anchor, and — when a model was given — a way to embed a description.
pub struct Host {
    index: Index,
    stamp: ModelStamp,
    toolset: Toolset,
    session: Session,
    embedder: Option<Box<dyn Embed>>,
    /// Every asset's acoustic features, built once: `search_music` judges candidates against them on every call.
    features: BTreeMap<u64, Features>,
}

impl Host {
    /// A host over `index`. `stamp` is the model the index was loaded against — the same one a model in `embedder` is
    /// stamped with — and `embedder` is `None` for a host with no model, whose `search_music` answers `unavailable`.
    pub fn new(index: Index, stamp: ModelStamp, embedder: Option<Box<dyn Embed>>) -> Host {
        let features = tools::features_by_hash(&index);
        Host { index, stamp, toolset: Toolset::default(), session: Session::new(), embedder, features }
    }

    /// How many assets the host can answer about.
    pub fn indexed(&self) -> usize {
        self.index.len()
    }

    /// Whether `search_music` can run.
    pub fn can_embed(&self) -> bool {
        self.embedder.is_some()
    }

    /// **Answer one input line**, whatever it is: a message goes to [`Host::handle`], and a line the framing refused is
    /// answered with the error it deserves, so the binary's loop has no case of its own.
    pub fn handle_line(&mut self, line: Line) -> Option<String> {
        match line {
            Line::Message(text) => self.handle(&text),
            Line::TooLong => Some(
                rpc_error(
                    Value::Null,
                    INVALID_REQUEST,
                    format!("a message is at most {MAX_LINE_BYTES} bytes"),
                )
                .to_string(),
            ),
            Line::NotUtf8 => Some(rpc_error(Value::Null, PARSE_ERROR, "a message is UTF-8 JSON").to_string()),
        }
    }

    /// **Answer one JSON-RPC message** with at most one line. A notification yields `None`. **Never panics on input**, and
    /// the line returned never contains a newline.
    pub fn handle(&mut self, line: &str) -> Option<String> {
        self.dispatch(line).map(|reply| reply.to_string())
    }

    fn dispatch(&mut self, line: &str) -> Option<Value> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(e) => return Some(rpc_error(Value::Null, PARSE_ERROR, format!("parse error: {e}"))),
        };
        let Value::Object(mut object) = message else {
            return Some(rpc_error(
                Value::Null,
                INVALID_REQUEST,
                "a message is one JSON object: a batch or a bare value is not one",
            ));
        };
        let method = match object.remove("method") {
            Some(Value::String(method)) => method,
            _ => {
                // A response to something this host never asked is not an error, only not for us.
                if object.contains_key("result") || object.contains_key("error") {
                    return None;
                }
                return Some(rpc_error(Value::Null, INVALID_REQUEST, "a request has a string `method`"));
            }
        };
        let id = match object.remove("id") {
            None => None,
            Some(id @ (Value::String(_) | Value::Number(_))) => Some(id),
            Some(_) => {
                return Some(rpc_error(Value::Null, INVALID_REQUEST, "`id` must be a string or a number"));
            }
        };
        // **A notification is never answered and never executed**, whatever method it names: a `tools/call` with no `id`
        // would otherwise run with no way to report what happened.
        let id = id?;
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(rpc_error(id, INVALID_REQUEST, "`jsonrpc` must be \"2.0\""));
        }
        let params = object.remove("params");
        let outcome = match method.as_str() {
            "initialize" => Self::initialize(params),
            "ping" => Ok(json!({})),
            "tools/list" => Self::list(params),
            "tools/call" => self.call(params),
            other => Err((METHOD_NOT_FOUND, format!("method not found: `{other}`"))),
        };
        Some(match outcome {
            Ok(result) => rpc_result(id, result),
            Err((code, message)) => rpc_error(id, code, message),
        })
    }

    fn initialize(params: Option<Value>) -> Result<Value, Protocol> {
        let requested = params.as_ref().and_then(|p| p.get("protocolVersion")).and_then(Value::as_str);
        let Some(requested) = requested else {
            return Err((INVALID_PARAMS, "`initialize` needs a string `protocolVersion`".to_owned()));
        };
        // *"If the server supports the requested protocol version, it MUST respond with the same version. Otherwise … another
        // protocol version it supports"*, which SHOULD be the latest.
        let version =
            PROTOCOL_VERSIONS.iter().copied().find(|v| *v == requested).unwrap_or(PROTOCOL_VERSIONS[0]);
        Ok(json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "siren-tools",
                "title": "bevy_siren audio tools",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "instructions": INSTRUCTIONS,
        }))
    }

    fn list(params: Option<Value>) -> Result<Value, Protocol> {
        // The list is one page. *"Invalid cursors SHOULD result in -32602"*, and every cursor is invalid here.
        if params.as_ref().and_then(|p| p.get("cursor")).is_some_and(|cursor| !cursor.is_null()) {
            return Err((INVALID_PARAMS, "this server's tool list is one page: omit `cursor`".to_owned()));
        }
        Ok(json!({ "tools": SPECS.iter().map(tool_json).collect::<Vec<_>>() }))
    }

    fn call(&mut self, params: Option<Value>) -> Result<Value, Protocol> {
        let params = params.unwrap_or(Value::Null);
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Err((INVALID_PARAMS, "`tools/call` needs a string `name`".to_owned()));
        };
        let Some(spec) = SPECS.iter().find(|spec| spec.name == name) else {
            return Err((INVALID_PARAMS, format!("Unknown tool: {name}")));
        };
        let given = match params.get("arguments") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => {
                return Ok(failure_result(&Failure::new(Code::BadArgument, "`arguments` must be an object")));
            }
        };
        // **Validated before any work**, against the row the schema was drawn from.
        let args = match spec.check(&given) {
            Ok(args) => args,
            Err(message) => return Ok(failure_result(&Failure::new(Code::BadArgument, message))),
        };
        Ok(match self.run(spec.name, &args) {
            Ok(structured) => tool_result(structured, false),
            Err(failure) => failure_result(&failure),
        })
    }

    fn run(&mut self, name: &str, args: &Args) -> Result<Value, Failure> {
        match name {
            "set_anchor" => self.set_anchor(args),
            "find_compatible_music" => self.find_compatible_music(args),
            "find_compatible_sfx" => self.find_compatible_sfx(args),
            "explain_match" => self.explain_match(args),
            "search_music" => self.search_music(args),
            "audition" => Err(Failure::new(
                Code::Unavailable,
                "`audition` plays through a running app, and this server has none: it never owns playback. Audition from a \
                 host that has an app, which calls the same library function in-process",
            )),
            other => Err(Failure::internal(format!("`{other}` is in SPECS but has no handler"))),
        }
    }

    fn set_anchor(&mut self, args: &Args) -> Result<Value, Failure> {
        let hash = require(args.id.get("asset_id").copied(), "asset_id")?;
        let anchor = self.session.set_anchor(&self.index, hash)?;
        Ok(json!({
            "anchor": {
                "asset_id": hex(anchor.hash),
                "dimension": anchor.embedding.len(),
                "features": features_json(&anchor.features),
                "provenance": provenance_json(&self.index.provenance.get(&hash).cloned().unwrap_or_default()),
            },
        }))
    }

    fn find_compatible_music(&self, args: &Args) -> Result<Value, Failure> {
        let k = require(args.k, "k")?;
        let answer =
            self.toolset.compatible_music(&self.index, &self.session, args.id.get("anchor").copied(), k)?;
        let summary = answer.describe();
        Ok(answer_json(&answer, summary))
    }

    fn find_compatible_sfx(&self, args: &Args) -> Result<Value, Failure> {
        let k = require(args.k, "k")?;
        let category = args.text.get("category").map(String::as_str);
        let answer = self.toolset.compatible_sfx(
            &self.index,
            &self.session,
            args.id.get("anchor").copied(),
            category,
            k,
        )?;
        let summary = answer.describe();
        Ok(answer_json(&answer, summary))
    }

    fn explain_match(&self, args: &Args) -> Result<Value, Failure> {
        let candidate = require(args.id.get("asset_id").copied(), "asset_id")?;
        let anchor = require(args.id.get("anchor").copied(), "anchor")?;
        // **The weighting that ranked the pair**, so the explanation's weights add up to the score the search returned.
        let (weighting, weights) = match args.choice.get("weighting").copied() {
            Some("music") | None => ("music", &self.toolset.music),
            Some("effects") => ("effects", &self.toolset.effects),
            Some(other) => {
                return Err(Failure::internal(format!("`weighting` is `{other}`, which has no handler")));
            }
        };
        let found = self.toolset.explain_match_with(&self.index, anchor, candidate, weights)?;
        Ok(json!({ "result": found_json(&found), "weighting": weighting, "weights": weights_json(weights) }))
    }

    fn search_music(&mut self, args: &Args) -> Result<Value, Failure> {
        let Some(embedder) = self.embedder.as_mut() else {
            return Err(Failure::new(
                Code::Unavailable,
                "`search_music` embeds the description with a model, and this server was started without one. Restart it \
                 with `--model <file.onnx> --vocab <tokenizer.json>`; `find_compatible_music` and `find_compatible_sfx` \
                 need no model",
            ));
        };
        let k = require(args.k, "k")?;
        let description = require(args.text.get("description"), "description")?;
        let query = Query::new(description).map_err(|e| Failure::new(Code::BadArgument, e.to_string()))?;
        // **An explicit anchor only.** `search_music` is documented as a pure read — the same description and index give
        // the same answer — and reading the session's anchor would make the same call return different results after
        // `set_anchor`.
        let anchor = match args.id.get("anchor").copied() {
            Some(hash) if self.index.records.contains_key(&hash) => hash,
            Some(hash) => return Err(ToolProblem::AnchorNotIndexed { anchor: hash }.into()),
            // No anchor: a hash the index does not hold, which `search_ranked` treats as having no acoustic features —
            // a semantic-only ranking, by its own documented rule.
            None => (0u64..).find(|h| !self.index.records.contains_key(h)).unwrap_or(0),
        };
        // **An empty index is answered before the model runs.** `tools::search_music` returns the empty answer without
        // reading the embedding, so there is nothing to embed, and a library with nothing in it is a state, not a mistake.
        let embedding = if self.index.is_empty() {
            Vec::new()
        } else {
            embedder
                .embed(&query)
                .map_err(|e| Failure::internal(format!("the model could not embed the description: {e}")))?
        };
        let request = Ranked {
            embedding: &embedding,
            anchor,
            features: &self.features,
            k,
            floor: Floor::default(),
            weights: &self.toolset.music,
        };
        let answer = tools::search_music(&self.index, &self.stamp, &query, &request)?;
        let summary = if self.index.is_empty() {
            String::from("the index is empty: nothing has been indexed in this library yet")
        } else {
            answer.describe()
        };
        Ok(answer_json(&answer, summary))
    }
}

/// A validated argument that must be there. Reaching the `Err` means `SPECS` and a handler disagree, which is a bug in this
/// module and is reported as one rather than unwrapped.
fn require<T>(value: Option<T>, name: &str) -> Result<T, Failure> {
    value.ok_or_else(|| {
        Failure::internal(format!(
            "`{name}` passed validation but is missing: SPECS and its handler disagree"
        ))
    })
}

/// One row of `tools/list`.
fn tool_json(spec: &ToolSpec) -> Value {
    // **Idempotency has one home**, `tools::TOOLS`; this looks it up by name rather than keeping a second copy. A row
    // missing there is treated as unsafe, the conservative answer, and a test fails.
    let idempotency = tools::TOOLS
        .iter()
        .find(|(name, _)| *name == spec.name)
        .map_or(Idempotency::Unsafe, |(_, idem)| *idem);
    json!({
        "name": spec.name,
        "description": format!("{} {}", spec.description, idempotency.describe(spec.name)),
        "inputSchema": spec.input_schema(),
        "annotations": {
            "readOnlyHint": spec.read_only,
            "destructiveHint": false,
            "idempotentHint": idempotency == Idempotency::Safe,
            "openWorldHint": false,
        },
    })
}
