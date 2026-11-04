//! **The tool host**: ADR 0006's `siren-tools` as a library, and as the process `#28` asks for.
//!
//! The host is driven as a client drives it — a line in, a line out — and the properties tested are the ones a client
//! relies on: the listing is the schema is the validation, a notification is never answered, no input breaks the framing,
//! and a bad call is a **result a model can read** rather than a protocol error it never sees.

#![cfg(feature = "semantic")]

use std::io::{Cursor, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::key::{Estimate, Key};
use bevy_siren::semantic::loudness::Loudness;
use bevy_siren::semantic::model::Model;
use bevy_siren::semantic::provenance::{Provenance, Source};
use bevy_siren::semantic::query::Query;
use bevy_siren::semantic::server::{
    Embed, Host, Line, MAX_LINE_BYTES, PROTOCOL_VERSIONS, SFX_DEFAULT_K, SPECS, read_message,
};
use bevy_siren::semantic::tempo::Tempo;
use bevy_siren::semantic::tokenizer::Vocabulary;
use bevy_siren::semantic::tools::{DEFAULT_K, Idempotency, MAX_K, TOOLS};
use bevy_siren::semantic::weights::Weights;
use serde_json::{Map, Value, json};

// ---------------------------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------------------------

fn stamp() -> ModelStamp {
    ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 }
}

/// Four assets in the xy-plane: 1 at 0°, 2 at 25°, 3 at 90°, and 4 at 25.5° — **a near-duplicate of 2** (cosine 0.9999, above
/// the 0.99 that makes two results variants of one sound). Asset 1 carries loudness, tempo and key so a breakdown has present
/// *and* missing signals; 2 and 3 carry provenance so a category can match.
fn index() -> Index {
    let mut index = Index::new(stamp());
    for (hash, vector) in [
        (1u64, vec![1.0, 0.0, 0.0]),
        (2, vec![0.906_307_8, 0.422_618_3, 0.0]),
        (3, vec![0.0, 1.0, 0.0]),
        (4, vec![0.902_630_3, 0.430_511_1, 0.0]),
    ] {
        index.insert(hash, vector).expect("inserts");
    }
    index.loudness.insert(
        1,
        Loudness {
            integrated: Some(-23.0),
            true_peak: -1.0,
            sample_peak: -1.5,
            blocks: 10,
            blocks_possible: 10,
        },
    );
    index.tempo.insert(1, Tempo { bpm: Some(120.0), confidence: 0.8, octave_ambiguous: false });
    index.key.insert(1, Estimate { key: Key { tonic: Some(0), major: true }, confidence: 0.6 });
    index.provenance.insert(
        2,
        Provenance {
            licence: Some("CC0-1.0".into()),
            source: Some(Source::Pack("Footsteps".into())),
            author: None,
        },
    );
    index.provenance.insert(
        3,
        Provenance {
            licence: Some("CC-BY-4.0".into()),
            source: Some(Source::Url("https://example.test/music".into())),
            author: Some("A. Composer".into()),
        },
    );
    index
}

/// An embedder that answers one fixed vector, so a search is deterministic without a model.
struct Fixed(Vec<f32>);

impl Embed for Fixed {
    fn embed(&mut self, _query: &Query) -> Result<Vec<f32>, String> {
        Ok(self.0.clone())
    }
}

/// An embedder that always fails, to prove a model fault is `internal` and an empty library never reaches it.
struct Broken;

impl Embed for Broken {
    fn embed(&mut self, _query: &Query) -> Result<Vec<f32>, String> {
        Err(String::from("the graph is gone"))
    }
}

fn host() -> Host {
    Host::new(index(), stamp(), Some(Box::new(Fixed(vec![1.0, 0.0, 0.0]))))
}

fn headless() -> Host {
    Host::new(index(), stamp(), None)
}

// ---------------------------------------------------------------------------------------------------------------
// Driving it like a client
// ---------------------------------------------------------------------------------------------------------------

fn request(id: i64, method: &str, params: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

/// One reply, which must be **one line of JSON** — the framing the transport requires.
fn reply(host: &mut Host, line: &str) -> Value {
    let out = host.handle(line).unwrap_or_else(|| panic!("`{line}` was not answered"));
    assert!(!out.contains('\n') && !out.contains('\r'), "a reply is one line: {out:?}");
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("a reply is JSON ({e}): {out}"))
}

/// A tool call's **result**. A call that reaches a tool is a result — never a JSON-RPC error, which is for the protocol.
fn call(host: &mut Host, tool: &str, arguments: &Value) -> Value {
    let r = reply(host, &request(1, "tools/call", &json!({ "name": tool, "arguments": arguments })));
    assert!(
        r.get("error").is_none(),
        "`{tool}` reached a tool, so it is a result, not a protocol error: {r}"
    );
    let result = r["result"].clone();
    // The specification: a tool returning structured content SHOULD also return it serialised in a text block.
    let text = result["content"][0]["text"].as_str().expect("a text block");
    assert_eq!(serde_json::from_str::<Value>(text).expect("the text is JSON"), result["structuredContent"]);
    result
}

fn ok(result: &Value) -> &Value {
    assert_eq!(result["isError"], false, "expected success: {result}");
    &result["structuredContent"]
}

/// `(code, message, retryable)` of a failed call.
fn failure(result: &Value) -> (String, String, bool) {
    assert_eq!(result["isError"], true, "expected a failure: {result}");
    let error = &result["structuredContent"]["error"];
    (
        error["code"].as_str().expect("a code").to_owned(),
        error["message"].as_str().expect("a message").to_owned(),
        error["retryable"].as_bool().expect("retryable"),
    )
}

fn id_of(n: u64) -> String {
    format!("{n:016x}")
}

/// A number off the wire, as the `f32` it was written from.
fn f32_of(value: &Value) -> f32 {
    value.as_f64().expect("a number") as f32
}

fn listed() -> Vec<Value> {
    let mut h = headless();
    let r = reply(&mut h, &request(1, "tools/list", &json!({})));
    r["result"]["tools"].as_array().expect("a list").clone()
}

// ---------------------------------------------------------------------------------------------------------------
// The listing, the schema and the validation are one table
// ---------------------------------------------------------------------------------------------------------------

/// **`#28`'s listing criterion**: the count, the names, and a schema each — and the names are `tools::TOOLS`', the one
/// place idempotency lives, so a tool added to either table without the other is this test failing.
#[test]
fn the_listing_names_the_six_tools_with_a_schema_each() {
    let tools = listed();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().expect("a name")).collect();
    assert_eq!(
        names,
        [
            "search_music",
            "set_anchor",
            "find_compatible_music",
            "find_compatible_sfx",
            "explain_match",
            "audition"
        ],
        "ADR 0007's six, in its order"
    );
    let rows: Vec<&str> = TOOLS.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, rows, "`SPECS` and `tools::TOOLS` list the same tools");
    assert_eq!(SPECS.len(), TOOLS.len());

    for tool in &tools {
        let name = tool["name"].as_str().expect("a name");
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}");
        assert_eq!(
            schema["additionalProperties"], false,
            "{name}: an unknown argument is refused, not ignored"
        );
        let properties = schema["properties"].as_object().expect("properties");
        assert!(!properties.is_empty(), "{name} takes arguments");
        for required in schema["required"].as_array().expect("required") {
            assert!(
                properties.contains_key(required.as_str().expect("a name")),
                "{name}: {required} is a property"
            );
        }
        let description = tool["description"].as_str().expect("a description");
        assert!(description.contains(name), "{name}'s description names the tool: {description}");
        assert!(description.contains("retry"), "{name}'s description states its idempotency: {description}");
        for (property, body) in properties {
            assert!(
                body["description"].as_str().is_some_and(|d| !d.is_empty()),
                "{name}.{property} is described"
            );
        }
    }
}

/// **The annotations say what the idempotency table says**: five tools are safe to retry and exactly one — the one that
/// starts a voice — is not; and only the tools that write state or play are not read-only.
#[test]
fn the_annotations_follow_the_idempotency_table() {
    let tools = listed();
    let hint = |name: &str, key: &str| -> bool {
        tools.iter().find(|t| t["name"] == name).expect("listed")["annotations"][key]
            .as_bool()
            .expect("a bool")
    };
    for (name, idempotency) in TOOLS {
        let safe = idempotency == Idempotency::Safe;
        assert_eq!(hint(name, "idempotentHint"), safe, "{name}");
        assert!(!hint(name, "destructiveHint"), "{name} destroys nothing");
        assert!(!hint(name, "openWorldHint"), "{name} reaches nothing outside the library");
    }
    assert_eq!(
        tools.iter().filter(|t| t["annotations"]["idempotentHint"] == false).count(),
        1,
        "exactly one tool is unsafe"
    );
    assert!(!hint("audition", "idempotentHint"));
    for (name, read_only) in [
        ("search_music", true),
        ("find_compatible_music", true),
        ("find_compatible_sfx", true),
        ("explain_match", true),
        ("set_anchor", false),
        ("audition", false),
    ] {
        assert_eq!(hint(name, "readOnlyHint"), read_only, "{name}");
    }
}

/// A valid value for every property a schema lists, **read from the schema** — so what is generated is what a client would
/// generate from the listing, not what the implementation happens to accept.
fn valid_arguments(schema: &Value) -> Map<String, Value> {
    let mut args = Map::new();
    for (name, property) in schema["properties"].as_object().expect("properties") {
        let value = match property["type"].as_str().expect("a type") {
            "integer" => property["minimum"].clone(),
            "string" if property.get("enum").is_some() => property["enum"][0].clone(),
            "string" if property.get("pattern").is_some() => json!(id_of(1)),
            "string" => json!("footsteps"),
            other => panic!("a schema type these tests do not know: {other}"),
        };
        args.insert(name.clone(), value);
    }
    args
}

/// Values that **violate** one property's own schema.
fn violations(property: &Value) -> Vec<Value> {
    match property["type"].as_str().expect("a type") {
        "integer" => {
            let max = property["maximum"].as_u64().expect("a maximum");
            vec![json!(0), json!(max + 1), json!(-1), json!(2.5), json!("3"), json!(true)]
        }
        "string" if property.get("pattern").is_some() => vec![
            json!(""),
            json!("zz"),
            json!("ABCDEF0123456789"),
            json!("00000000000000012"),
            json!("/music/a.wav"),
            json!(7),
            json!(true),
        ],
        "string" if property.get("enum").is_some() => {
            vec![json!("nonsense"), json!(""), json!("MUSIC"), json!(7), json!(false)]
        }
        _ => vec![json!(""), json!("   "), json!(7), json!(false)],
    }
}

/// **The schema and the validator are one table**, proved from outside: every value a schema allows is accepted by the
/// call, every value it forbids is `bad_argument` naming the property, and every required property left out is named. If
/// the check and the schema were two things this would be the test that found them disagreeing.
#[test]
fn the_schema_is_what_the_call_enforces() {
    for tool in listed() {
        let name = tool["name"].as_str().expect("a name").to_owned();
        let schema = &tool["inputSchema"];
        let valid = valid_arguments(schema);
        let mut h = host();

        // 1. What the schema allows is never refused as a bad argument. (`audition` is `unavailable`, which is the
        //    capability's answer and a different thing.)
        let accepted = call(&mut h, &name, &Value::Object(valid.clone()));
        if accepted["isError"] == true {
            let (code, message, _) = failure(&accepted);
            assert_ne!(code, "bad_argument", "{name} refused its own schema's valid arguments: {message}");
            assert_ne!(code, "internal", "{name}: SPECS and its handler disagree: {message}");
        }

        // 1b. **The bounds the schema states are accepted** — its own `minimum` and `maximum`. A schema that said 101 while
        //     the check stopped at 100 would pass every violation below and still be wrong.
        for (property, body) in schema["properties"].as_object().expect("properties") {
            // **Every word an enum allows is accepted**, not only the first one the generator happens to pick.
            if let Some(options) = body["enum"].as_array() {
                for option in options {
                    let mut args = valid.clone();
                    args.insert(property.clone(), option.clone());
                    let result = call(&mut h, &name, &Value::Object(args));
                    if result["isError"] == true {
                        assert_ne!(
                            failure(&result).0,
                            "bad_argument",
                            "{name}.{property} refused its own value {option}"
                        );
                    }
                }
            }
            if body["type"] != "integer" {
                continue;
            }
            for bound in ["minimum", "maximum"] {
                let mut args = valid.clone();
                args.insert(property.clone(), body[bound].clone());
                let result = call(&mut h, &name, &Value::Object(args));
                if result["isError"] == true {
                    assert_ne!(
                        failure(&result).0,
                        "bad_argument",
                        "{name}.{property} refused its own {bound}"
                    );
                }
            }
        }
        // 2. What it forbids is refused, naming the property, as something a model can fix.
        for (property, body) in schema["properties"].as_object().expect("properties") {
            for bad in violations(body) {
                let mut args = valid.clone();
                args.insert(property.clone(), bad.clone());
                let (code, message, retryable) = failure(&call(&mut h, &name, &Value::Object(args)));
                assert_eq!(code, "bad_argument", "{name}.{property} = {bad}: {message}");
                assert!(retryable, "{name}.{property}: a bad argument is fixable by the caller");
                assert!(
                    message.contains(property.as_str()),
                    "{name}.{property} = {bad}: the message names it: {message}"
                );
            }
        }

        // 3. A required property left out is named.
        for required in schema["required"].as_array().expect("required") {
            let missing = required.as_str().expect("a name");
            let mut args = valid.clone();
            args.remove(missing);
            let (code, message, _) = failure(&call(&mut h, &name, &Value::Object(args)));
            assert_eq!(code, "bad_argument", "{name} without {missing}");
            assert!(message.contains(missing) && message.contains("required"), "{name}: {message}");
        }

        // 4. An argument the tool does not take is refused, naming what it does take.
        let mut args = valid.clone();
        args.insert("nonsense".into(), json!(1));
        let (code, message, _) = failure(&call(&mut h, &name, &Value::Object(args)));
        assert_eq!(code, "bad_argument", "{name}");
        assert!(message.contains("nonsense"), "{message}");
        let first = schema["properties"].as_object().expect("properties").keys().next().expect("one").clone();
        assert!(message.contains(&format!("`{first}`")), "{name}'s refusal lists what it takes: {message}");
    }
}

/// **Every problem in one message**, so an agent with two mistakes fixes both in one retry.
#[test]
fn every_problem_is_reported_at_once() {
    let mut h = host();
    let (code, message, retryable) =
        failure(&call(&mut h, "explain_match", &json!({ "asset_id": "zz", "extra": 1 })));
    assert_eq!(code, "bad_argument");
    assert!(retryable);
    for needle in ["extra", "asset_id", "anchor"] {
        assert!(message.contains(needle), "{needle} is in: {message}");
    }
}

/// `3.0` is an integer to JSON Schema, so a model that writes it has made no mistake the schema forbids.
#[test]
fn a_whole_number_written_with_a_point_is_an_integer() {
    let mut h = host();
    let result = call(&mut h, "search_music", &json!({ "description": "x", "k": 2.0 }));
    assert_eq!(ok(&result)["results"].as_array().expect("results").len(), 2);
}

// ---------------------------------------------------------------------------------------------------------------
// The protocol
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn initialize_echoes_a_supported_version_and_answers_its_own_otherwise() {
    let mut h = headless();
    for version in PROTOCOL_VERSIONS {
        let r = reply(
            &mut h,
            &request(1, "initialize", &json!({ "protocolVersion": version, "capabilities": {} })),
        );
        assert_eq!(r["result"]["protocolVersion"], version, "a supported version is answered with itself");
    }
    let r = reply(&mut h, &request(2, "initialize", &json!({ "protocolVersion": "1.0.0" })));
    assert_eq!(
        r["result"]["protocolVersion"], PROTOCOL_VERSIONS[0],
        "an unsupported one gets this server's latest"
    );
    assert!(r["result"]["capabilities"]["tools"].is_object(), "a server with tools declares the capability");
    assert_eq!(r["result"]["serverInfo"]["name"], "siren-tools");
    assert!(r["result"]["instructions"].as_str().is_some_and(|i| i.contains("set_anchor")));

    let r = reply(&mut h, &request(3, "initialize", &json!({})));
    assert_eq!(r["error"]["code"], -32602, "no version is an invalid parameter: {r}");
}

/// **A notification is never answered, and never executed**: a `tools/call` sent with no `id` would otherwise run with no
/// way to report what happened. Proved by the state it must not change.
#[test]
fn a_notification_is_never_answered_or_run() {
    let mut h = host();
    assert_eq!(h.handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#), None);
    assert_eq!(h.handle(r#"{"jsonrpc":"2.0","method":"nonsense/never"}"#), None, "even an unknown one");
    let as_notification = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": { "name": "set_anchor", "arguments": { "asset_id": id_of(1) } },
    })
    .to_string();
    assert_eq!(h.handle(&as_notification), None);
    let (code, message, _) = failure(&call(&mut h, "find_compatible_music", &json!({})));
    assert_eq!(code, "refused", "the notification set no anchor: {message}");
}

#[test]
fn ping_and_the_ids_that_come_back() {
    let mut h = headless();
    assert_eq!(reply(&mut h, &request(7, "ping", &json!({})))["result"], json!({}));
    for id in [json!(0), json!(-4), json!("abc"), json!(9_007_199_254_740_991u64)] {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": "ping" }).to_string();
        assert_eq!(reply(&mut h, &line)["id"], id, "the id comes back exactly as sent");
    }
    let r = reply(&mut h, &request(8, "resources/list", &json!({})));
    assert_eq!(r["error"]["code"], -32601, "an unknown method: {r}");
    assert_eq!(r["id"], 8);
}

#[test]
fn an_unknown_tool_is_a_protocol_error_and_a_bad_call_shape_is_too() {
    let mut h = headless();
    let r = reply(&mut h, &request(1, "tools/call", &json!({ "name": "nope", "arguments": {} })));
    assert_eq!(r["error"]["code"], -32602, "the specification's own example for an unknown tool");
    assert!(r["error"]["message"].as_str().is_some_and(|m| m.contains("Unknown tool: nope")), "{r}");
    let r = reply(&mut h, &request(2, "tools/call", &json!({ "arguments": {} })));
    assert_eq!(r["error"]["code"], -32602, "no name: {r}");
    let r = reply(&mut h, &request(3, "tools/call", &json!({ "name": 7 })));
    assert_eq!(r["error"]["code"], -32602, "a name that is not a string: {r}");
    // `arguments` may be omitted or null; a non-object is a bad argument the model can read, not a protocol error.
    let r = reply(&mut h, &request(4, "tools/call", &json!({ "name": "audition" })));
    assert!(r.get("error").is_none() && r["result"]["isError"] == true, "{r}");
    let result = call(&mut h, "set_anchor", &json!([1]));
    assert_eq!(failure(&result).0, "bad_argument");
}

#[test]
fn the_list_is_one_page_so_a_cursor_is_invalid() {
    let mut h = headless();
    let r = reply(&mut h, &request(1, "tools/list", &json!({ "cursor": "next" })));
    assert_eq!(r["error"]["code"], -32602, "{r}");
    let r = reply(&mut h, &request(2, "tools/list", &json!({ "cursor": null })));
    assert!(r["result"]["tools"].is_array(), "a null cursor is no cursor: {r}");
    let r = reply(&mut h, r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
    assert!(r["result"]["tools"].is_array(), "params may be omitted: {r}");
}

/// What is not JSON-RPC is answered as JSON-RPC says, with an `id` of `null` because none can be trusted.
#[test]
fn what_is_not_a_request_gets_the_error_json_rpc_names() {
    let mut h = headless();
    for (line, code) in [
        ("not json", -32700),
        ("{", -32700),
        ("", -32700),
        ("[]", -32600),
        (r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#, -32600),
        ("5", -32600),
        ("null", -32600),
        (r#"{"jsonrpc":"2.0","id":1}"#, -32600),
        (r#"{"jsonrpc":"2.0","id":1,"method":42}"#, -32600),
        (r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#, -32600),
        (r#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#, -32600),
        (r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#, -32600),
    ] {
        let r = reply(&mut h, line);
        assert_eq!(r["error"]["code"], code, "`{line}` → {r}");
        assert_eq!(r["id"], Value::Null, "`{line}`: no trustworthy id");
    }
    // A request that names its id but not its version is answered under that id.
    let r = reply(&mut h, r#"{"id":9,"method":"ping"}"#);
    assert_eq!((r["error"]["code"].clone(), r["id"].clone()), (json!(-32600), json!(9)));
    // A *response* to something this host never asked is not for it: no reply, no error.
    assert_eq!(h.handle(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#), None);
    assert_eq!(h.handle(r#"{"jsonrpc":"2.0","id":1,"error":{"code":1,"message":"x"}}"#), None);
}

/// **No input breaks the framing or panics.** Each line is answered with one line of valid JSON, or not at all — and never
/// with a newline in it, which would make the next message unreadable to the client.
#[test]
fn no_input_breaks_the_framing_or_panics() {
    let deep = format!("{}1{}", "[".repeat(10_000), "]".repeat(10_000));
    let deep_object = format!("{}{}", "{\"a\":".repeat(5_000), "1");
    let hostile: Vec<String> = vec![
        String::new(),
        " ".into(),
        "\t\u{0}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"set_anchor\",\"arguments\":{\"asset_id\":\"\\ud800\"}}}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"find_compatible_sfx\",\"arguments\":{\"category\":\"a\\nb\\r\\u2028\"}}}".into(),
        deep,
        deep_object,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":null}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"search_music\",\"arguments\":{\"description\":\"\u{1f3b5}\u{0}\",\"k\":1e308}}}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"search_music\",\"arguments\":{\"description\":\"x\",\"k\":18446744073709551615}}}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"search_music\",\"arguments\":{\"description\":\"x\",\"k\":-1e999}}}".into(),
        "\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}".into(),
        "x".repeat(3_000_000),
    ];
    let mut h = host();
    for line in &hostile {
        let shown: String = line.chars().take(60).collect();
        match h.handle(line) {
            None => {}
            Some(out) => {
                assert!(!out.contains('\n') && !out.contains('\r'), "`{shown}` broke the framing: {out:?}");
                let parsed: Value =
                    serde_json::from_str(&out).unwrap_or_else(|e| panic!("`{shown}` → not JSON ({e})"));
                assert_eq!(parsed["jsonrpc"], "2.0", "`{shown}`");
                assert!(parsed.get("result").is_some() ^ parsed.get("error").is_some(), "`{shown}` → {out}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The framing that reads lines
// ---------------------------------------------------------------------------------------------------------------

fn lines(bytes: &[u8], max: usize) -> Vec<Line> {
    let mut reader = Cursor::new(bytes.to_vec());
    let mut out = Vec::new();
    while let Some(line) = read_message(&mut reader, max).expect("a cursor does not fail") {
        out.push(line);
    }
    out
}

fn message(text: &str) -> Line {
    Line::Message(text.to_owned())
}

#[test]
fn lines_are_split_on_newlines_and_blank_ones_skipped() {
    assert_eq!(lines(b"a\nb\r\n\n   \n\r\nc", 100), [message("a"), message("b"), message("c")]);
    assert_eq!(lines(b"", 100), []);
    assert_eq!(lines(b"\n\n", 100), []);
    assert_eq!(lines(b"only", 100), [message("only")], "a last line with no newline is still a line");
}

/// **A line over the ceiling is drained through its newline**, so the stream stays aligned for the next message — the
/// difference between a refused message and a corrupted session.
#[test]
fn an_over_long_line_is_drained_and_the_next_one_still_reads() {
    let mut input = vec![b'x'; 50];
    input.push(b'\n');
    input.extend_from_slice(b"next\n");
    assert_eq!(
        lines(&input, 50),
        [message(&"x".repeat(50)), message("next")],
        "exactly the ceiling is accepted"
    );
    assert_eq!(lines(&input, 49), [Line::TooLong, message("next")], "one byte over is refused and drained");
    let huge = [vec![b'y'; 2_000_000], b"\nping\n".to_vec()].concat();
    assert_eq!(lines(&huge, MAX_LINE_BYTES), [Line::TooLong, message("ping")]);
    // Over-long and unterminated: refused, and then the stream simply ends.
    assert_eq!(lines(&[b'z'; 200], 10), [Line::TooLong]);
}

#[test]
fn a_line_that_is_not_utf8_is_refused_and_the_stream_continues() {
    assert_eq!(lines(b"ok\n\xff\xfe\nafter\n", 100), [message("ok"), Line::NotUtf8, message("after")]);
}

/// The refusals the framing produces are answered as JSON-RPC, so the binary's loop has no case of its own.
#[test]
fn a_refused_line_is_answered_with_the_right_error() {
    let mut h = headless();
    let too_long = h.handle_line(Line::TooLong).expect("answered");
    assert_eq!(serde_json::from_str::<Value>(&too_long).expect("JSON")["error"]["code"], -32600);
    let not_utf8 = h.handle_line(Line::NotUtf8).expect("answered");
    assert_eq!(serde_json::from_str::<Value>(&not_utf8).expect("JSON")["error"]["code"], -32700);
}

// ---------------------------------------------------------------------------------------------------------------
// The tools, against ADR 0007's shapes
// ---------------------------------------------------------------------------------------------------------------

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value.as_object().expect("an object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

/// **ADR 0007's result shape, field for field** — it is what an agent's prompt is built against.
#[test]
fn a_list_result_has_the_shape_the_adr_fixes() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let result = call(&mut h, "find_compatible_music", &json!({}));
    let answer = ok(&result);
    assert_eq!(keys(answer), ["results", "summary", "tally", "weights"]);
    let first = &answer["results"][0];
    assert_eq!(keys(first), ["asset_id", "breakdown", "explain", "provenance", "score", "similarity"]);
    assert_eq!(
        first["asset_id"],
        id_of(2).as_str(),
        "the nearest neighbour of 1, with its near-duplicate 4 reported"
    );
    assert!(first["score"].is_number() && first["similarity"].is_number());
    assert!(first["explain"].as_str().is_some_and(|e| !e.is_empty()));
    assert_eq!(keys(&first["provenance"]), ["author", "licence", "source"]);
    assert_eq!(first["provenance"]["source"], "pack:Footsteps");
    assert_eq!(first["provenance"]["licence"], "CC0-1.0");
    assert_eq!(first["provenance"]["author"], Value::Null, "absence is null");

    let parts = first["breakdown"]["parts"].as_array().expect("parts");
    let signals: Vec<&str> = parts.iter().map(|p| p["signal"].as_str().expect("a signal")).collect();
    assert_eq!(signals, ["semantic", "acoustic", "tempo", "key"], "the order a breakdown lists them");
    for part in parts {
        assert_eq!(keys(part), ["contribution", "missing", "raw", "signal", "weight"]);
        // **A missing signal is `null`, never `0`.**
        assert_eq!(part["raw"].is_null(), part["missing"] == true, "{part}");
    }
    assert_eq!(parts[0]["missing"], false, "similarity is always there");
    assert_eq!(parts[2]["missing"], true, "asset 2 has no tempo");
    assert_eq!(parts[2]["raw"], Value::Null);

    assert_eq!(keys(&answer["tally"]), ["below_floor", "excluded", "returned", "variants", "wrong_category"]);
    assert_eq!(
        answer["weights"].as_object().expect("weights").len(),
        10,
        "every field of `Weights`, which is destructured exhaustively"
    );
    assert_eq!(f32_of(&answer["weights"]["semantic"]), Weights::default().semantic);
}

/// **`asset_id` is 16 lowercase hex wherever one appears**, never a path and never a decimal.
#[test]
fn an_asset_id_is_sixteen_lowercase_hex_everywhere() {
    fn walk(value: &Value, found: &mut usize) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    if key == "asset_id" || key == "resembles" {
                        let id = inner.as_str().unwrap_or_else(|| panic!("{key} is a string: {inner}"));
                        assert!(
                            id.len() == 16
                                && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                            "{id}"
                        );
                        *found += 1;
                    }
                    walk(inner, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|i| walk(i, found)),
            _ => {}
        }
    }
    let mut h = host();
    let mut found = 0;
    for (tool, args) in [
        ("set_anchor", json!({ "asset_id": id_of(1) })),
        ("find_compatible_music", json!({ "k": 10 })),
        ("find_compatible_sfx", json!({ "k": 10 })),
        ("explain_match", json!({ "asset_id": id_of(2), "anchor": id_of(1) })),
        ("search_music", json!({ "description": "x", "k": 10 })),
    ] {
        walk(ok(&call(&mut h, tool, &args)), &mut found);
    }
    assert!(found >= 10, "the walk reached the ids ({found})");
}

/// A number on the wire is the **shortest decimal that reads back as the same `f32`**: `0.683` and not the
/// `0.6830000281333923` that widening it would print. Scanned in the raw text, because a parsed `Value` cannot show it.
#[test]
fn numbers_are_written_as_the_f32_they_are() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let line = h
        .handle(&request(
            1,
            "tools/call",
            &json!({ "name": "find_compatible_music", "arguments": { "k": 10 } }),
        ))
        .expect("answered");
    let mut longest = 0;
    let mut run = 0;
    let mut after_point = false;
    for ch in line.chars() {
        match ch {
            '.' => after_point = true,
            d if d.is_ascii_digit() && after_point => {
                run += 1;
                longest = longest.max(run);
            }
            _ => {
                run = 0;
                after_point = false;
            }
        }
    }
    // A widened `f64` of a typical `f32` prints 15–17 digits after the point; the shortest `f32` decimal prints at most 9
    // significant digits, and a few more only for a value so small that leading zeros follow the point.
    assert!(
        longest <= 12,
        "a number was written with {longest} digits after the point: the f32 was widened to f64"
    );
}

#[test]
fn set_anchor_says_what_the_library_knows() {
    let mut h = host();
    let result = call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) }));
    let anchor = &ok(&result)["anchor"];
    assert_eq!(anchor["asset_id"], id_of(1).as_str());
    assert_eq!(anchor["dimension"], 3);
    assert_eq!(keys(&anchor["features"]), ["key", "loudness", "spectral", "tempo"]);
    assert_eq!(anchor["features"]["loudness"]["integrated_lufs"], -23.0);
    assert_eq!(anchor["features"]["tempo"]["bpm"], 120.0);
    assert_eq!(anchor["features"]["key"]["name"], "C major");
    assert_eq!(anchor["features"]["spectral"], Value::Null, "never measured is null");
    // Provenance for an asset with none is all-null, not absent.
    assert_eq!(keys(&anchor["provenance"]), ["author", "licence", "source"]);
    assert_eq!(anchor["provenance"]["licence"], Value::Null);
}

/// A tempo that was **judged and found absent** is an object with a `null` inside; one never measured is `null`. Different
/// facts, and the wire keeps them different.
#[test]
fn a_non_rhythmic_clip_is_not_the_same_as_an_unmeasured_one() {
    let mut index = index();
    index.tempo.insert(2, Tempo { bpm: None, confidence: 0.1, octave_ambiguous: false });
    let mut h = Host::new(index, stamp(), None);
    let judged = call(&mut h, "set_anchor", &json!({ "asset_id": id_of(2) }));
    let unmeasured = call(&mut h, "set_anchor", &json!({ "asset_id": id_of(3) }));
    assert_eq!(ok(&judged)["anchor"]["features"]["tempo"], json!({ "bpm": null, "confidence": 0.1 }));
    assert_eq!(ok(&unmeasured)["anchor"]["features"]["tempo"], Value::Null);
}

// ---------------------------------------------------------------------------------------------------------------
// State: the anchor belongs to the host
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn an_anchor_is_used_when_none_is_passed_and_overridden_when_one_is() {
    let mut h = host();
    // **No anchor, none passed**: a refusal that says how to set one, and says it is not the caller's mistake.
    let (code, message, retryable) = failure(&call(&mut h, "find_compatible_music", &json!({})));
    assert_eq!((code.as_str(), retryable), ("refused", false), "the state must change, not the call");
    assert!(message.contains("set_anchor"), "{message}");

    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let from_session = call(&mut h, "find_compatible_music", &json!({ "k": 10 }));
    let explicit = call(&mut h, "find_compatible_music", &json!({ "k": 10, "anchor": id_of(1) }));
    assert_eq!(from_session, explicit, "the session's anchor is the default, and naming it changes nothing");

    // A different anchor changes the neighbours — and the explicit one wins over the session's without replacing it.
    let other = call(&mut h, "find_compatible_music", &json!({ "k": 10, "anchor": id_of(3) }));
    assert_ne!(ok(&other)["results"][0]["asset_id"], ok(&from_session)["results"][0]["asset_id"]);
    assert_eq!(
        call(&mut h, "find_compatible_music", &json!({ "k": 10 })),
        from_session,
        "the session is unchanged"
    );
}

/// `#42`'s idempotency criterion, over the wire: the same anchor twice leaves every later answer field-for-field equal.
#[test]
fn setting_the_same_anchor_twice_changes_nothing() {
    let mut h = host();
    let first = call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) }));
    let before = call(&mut h, "find_compatible_sfx", &json!({}));
    let second = call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) }));
    let after = call(&mut h, "find_compatible_sfx", &json!({}));
    assert_eq!(first, second);
    assert_eq!(before, after);
}

/// **Retrying every read-only tool gives an identical reply**, byte for byte — what `idempotentHint` promises.
#[test]
fn a_retried_read_is_byte_identical() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    for (tool, args) in [
        ("search_music", json!({ "description": "x", "k": 4 })),
        ("find_compatible_music", json!({})),
        ("find_compatible_sfx", json!({ "category": "foot" })),
        ("explain_match", json!({ "asset_id": id_of(2), "anchor": id_of(1) })),
    ] {
        let line = request(1, "tools/call", &json!({ "name": tool, "arguments": args }));
        let once = h.handle(&line).expect("answered");
        let twice = h.handle(&line).expect("answered");
        assert_eq!(once, twice, "{tool}");
    }
}

#[test]
fn two_hosts_share_no_anchor() {
    let (mut a, mut b) = (host(), host());
    ok(&call(&mut a, "set_anchor", &json!({ "asset_id": id_of(1) })));
    ok(&call(&mut b, "set_anchor", &json!({ "asset_id": id_of(3) })));
    let from_a = call(&mut a, "find_compatible_music", &json!({ "k": 1 }));
    let from_b = call(&mut b, "find_compatible_music", &json!({ "k": 1 }));
    assert_ne!(
        ok(&from_a)["results"][0]["asset_id"],
        ok(&from_b)["results"][0]["asset_id"],
        "each has its own anchor"
    );
}

#[test]
fn an_anchor_outside_the_index_is_refused_naming_the_hash() {
    let mut h = host();
    let (code, message, retryable) = failure(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(99) })));
    assert_eq!((code.as_str(), retryable), ("refused", false));
    assert!(message.contains(&id_of(99)), "{message}");
    // …and the failed call did not set one.
    assert_eq!(failure(&call(&mut h, "find_compatible_music", &json!({}))).0, "refused");
}

// ---------------------------------------------------------------------------------------------------------------
// find_compatible_*, explain_match
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn music_and_effects_are_weighted_differently_and_say_so() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let music = call(&mut h, "find_compatible_music", &json!({}));
    let effects = call(&mut h, "find_compatible_sfx", &json!({}));
    // **Compared as the `f32` they are**: `json!(1.2f32)` widens to `1.2000000476837158`, which is the thing the wire form
    // exists not to print.
    assert_eq!(f32_of(&ok(&music)["weights"]["acoustic"]), Weights::default().acoustic);
    assert_eq!(f32_of(&ok(&effects)["weights"]["acoustic"]), Weights::effects().acoustic);
    assert_eq!(f32_of(&ok(&effects)["weights"]["tempo"]), 0.0, "an effect has no tempo to weigh");
}

#[test]
fn the_default_k_is_three_for_music_and_six_for_effects() {
    assert_eq!((DEFAULT_K, SFX_DEFAULT_K), (3, 6));
    let mut index = Index::new(stamp());
    for hash in 1..=20u64 {
        // **Spaced 0.15 rad apart**, so the cosine between neighbours (0.9888) stays under the 0.99 that makes two results
        // variants of one sound, and near-duplicate suppression leaves every candidate in place.
        let angle = hash as f32 * 0.15;
        index.insert(hash, vec![angle.cos(), angle.sin(), 0.0]).expect("inserts");
    }
    let mut h = Host::new(index, stamp(), None);
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let music = call(&mut h, "find_compatible_music", &json!({}));
    let sfx = call(&mut h, "find_compatible_sfx", &json!({}));
    assert_eq!(ok(&music)["results"].as_array().expect("results").len(), DEFAULT_K);
    assert_eq!(ok(&sfx)["results"].as_array().expect("results").len(), SFX_DEFAULT_K);
    let many = call(&mut h, "find_compatible_sfx", &json!({ "k": MAX_K }));
    // Assets 2–11 are within a quarter turn of the anchor; 12 and beyond have a negative cosine and fall under the floor.
    // **`k` is a bound, not a quota**: asking for 100 returns what exists.
    assert_eq!(ok(&many)["results"].as_array().expect("results").len(), 10);
}

#[test]
fn a_category_filters_and_a_near_duplicate_is_reported() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let footsteps = call(&mut h, "find_compatible_sfx", &json!({ "category": "FOOTSTEPS", "k": 10 }));
    let answer = ok(&footsteps);
    let ids: Vec<&str> = answer["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|r| r["asset_id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, [id_of(2)], "only the asset whose source says footsteps, whatever the case");
    assert_eq!(answer["tally"]["wrong_category"], 2, "the filter is counted, not silent");
    assert!(
        answer["summary"].as_str().is_some_and(|s| s.contains("outside the category")),
        "{}",
        answer["summary"]
    );

    let all = call(&mut h, "find_compatible_sfx", &json!({ "k": 10 }));
    let variants = ok(&all)["tally"]["variants"].as_array().expect("variants");
    assert_eq!(variants.len(), 1, "asset 4 is a near-duplicate of 2 and is reported rather than returned");
    assert_eq!(variants[0]["asset_id"], id_of(4).as_str());
    assert_eq!(variants[0]["resembles"], id_of(2).as_str(), "it resembles the better-scoring kept one");
}

/// **`explain_match` gives the numbers the search gave** for that pair, because both call the same breakdown.
#[test]
fn explaining_a_match_agrees_with_the_search_that_ranked_it() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let search = call(&mut h, "find_compatible_music", &json!({ "k": 10 }));
    let found = ok(&search)["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|r| r["asset_id"] == id_of(2).as_str())
        .expect("asset 2 was ranked")
        .clone();
    let explained = call(&mut h, "explain_match", &json!({ "asset_id": id_of(2), "anchor": id_of(1) }));
    assert_eq!(ok(&explained)["result"], found);
    assert!(ok(&explained)["weights"].is_object());

    let (code, message, _) =
        failure(&call(&mut h, "explain_match", &json!({ "asset_id": id_of(99), "anchor": id_of(1) })));
    assert_eq!(code, "refused");
    assert!(message.contains(&id_of(99)), "{message}");
}

/// **An explanation under the wrong weighting does not add up to the search it explains.** The first real session through
/// the binary showed it: `find_compatible_sfx` scored a pair with `acoustic` at 1.2 and `explain_match` of the same pair
/// said 0.5, because the library's explanation was hard-wired to the music weighting. `weighting` is the fix, and the two
/// halves of this test are the evidence: under `effects` the explanation **is** the search's result, and under the default
/// it is not.
#[test]
fn explaining_an_effect_uses_the_weighting_that_ranked_it() {
    let mut h = host();
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    let search = call(&mut h, "find_compatible_sfx", &json!({ "k": 10 }));
    let found = ok(&search)["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|r| r["asset_id"] == id_of(2).as_str())
        .expect("asset 2 was ranked")
        .clone();

    let args = json!({ "asset_id": id_of(2), "anchor": id_of(1), "weighting": "effects" });
    let effects = call(&mut h, "explain_match", &args);
    assert_eq!(
        ok(&effects)["result"],
        found,
        "under the weighting that ranked it, the explanation is the result"
    );
    assert_eq!(ok(&effects)["weighting"], "effects");
    assert_eq!(f32_of(&ok(&effects)["weights"]["acoustic"]), Weights::effects().acoustic);

    let music = call(&mut h, "explain_match", &json!({ "asset_id": id_of(2), "anchor": id_of(1) }));
    assert_eq!(ok(&music)["weighting"], "music", "music is the default");
    assert_ne!(
        ok(&music)["result"],
        found,
        "under the other weighting the numbers differ, which is the defect"
    );
    assert_eq!(f32_of(&ok(&music)["weights"]["acoustic"]), Weights::default().acoustic);

    let (code, message, retryable) = failure(&call(
        &mut h,
        "explain_match",
        &json!({ "asset_id": id_of(2), "anchor": id_of(1), "weighting": "orchestral" }),
    ));
    assert_eq!((code.as_str(), retryable), ("bad_argument", true));
    assert!(
        message.contains("`music`") && message.contains("`effects`"),
        "the allowed words are named: {message}"
    );
}

// ---------------------------------------------------------------------------------------------------------------
// search_music, audition
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn search_music_ranks_by_the_embedding_and_bounds_k() {
    let mut h = host();
    let result = call(&mut h, "search_music", &json!({ "description": "a low rumble" }));
    let answer = ok(&result);
    let results = answer["results"].as_array().expect("results");
    assert_eq!(results.len(), DEFAULT_K, "k defaults to 3 when omitted");
    assert_eq!(results[0]["asset_id"], id_of(1).as_str(), "the embedding is asset 1's own vector");
    assert_eq!(results[0]["similarity"], 1.0);
    let scores: Vec<f64> = results.iter().map(|r| r["score"].as_f64().expect("a score")).collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "best first: {scores:?}");

    let two = call(&mut h, "search_music", &json!({ "description": "x", "k": 2 }));
    assert_eq!(ok(&two)["results"].as_array().expect("results").len(), 2);

    for k in [0, MAX_K + 1] {
        let (code, message, retryable) =
            failure(&call(&mut h, "search_music", &json!({ "description": "x", "k": k })));
        assert_eq!((code.as_str(), retryable), ("bad_argument", true), "k = {k}");
        assert!(message.contains("between 1 and 100"), "the valid range is named: {message}");
    }
}

/// **`search_music` is a pure read.** It must not read the session's anchor: otherwise the same call would answer
/// differently after `set_anchor`, and the idempotency the listing promises would be false.
#[test]
fn search_music_ignores_the_session_anchor_and_takes_an_explicit_one() {
    let mut h = host();
    let before = call(&mut h, "search_music", &json!({ "description": "x", "k": 4 }));
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(3) })));
    let after = call(&mut h, "search_music", &json!({ "description": "x", "k": 4 }));
    assert_eq!(before, after, "a session's anchor is not an input to a pure read");
    let explicit = call(&mut h, "search_music", &json!({ "description": "x", "k": 4, "anchor": id_of(3) }));
    assert_eq!(
        ok(&explicit)["results"].as_array().expect("results").len(),
        4,
        "an explicit anchor is accepted"
    );
    let (code, message, _) =
        failure(&call(&mut h, "search_music", &json!({ "description": "x", "anchor": id_of(99) })));
    assert_eq!(code, "refused");
    assert!(message.contains(&id_of(99)), "{message}");
}

/// **With no model the tool says what is missing**, and the tools that need no model still work.
#[test]
fn search_music_without_a_model_is_unavailable_and_says_how_to_fix_it() {
    let mut h = headless();
    assert!(!h.can_embed());
    let (code, message, retryable) = failure(&call(&mut h, "search_music", &json!({ "description": "x" })));
    assert_eq!((code.as_str(), retryable), ("unavailable", false));
    assert!(message.contains("--model") && message.contains("--vocab"), "{message}");
    ok(&call(&mut h, "set_anchor", &json!({ "asset_id": id_of(1) })));
    ok(&call(&mut h, "find_compatible_music", &json!({})));
}

#[test]
fn a_model_fault_is_internal_and_an_empty_library_never_reaches_the_model() {
    let mut broken = Host::new(index(), stamp(), Some(Box::new(Broken)));
    let (code, message, retryable) =
        failure(&call(&mut broken, "search_music", &json!({ "description": "x" })));
    assert_eq!((code.as_str(), retryable), ("internal", false));
    assert!(message.contains("the graph is gone"), "{message}");

    // `#29`: an empty library is an empty list **with a reason**, not an error — and the model is not asked.
    let mut empty = Host::new(Index::new(stamp()), stamp(), Some(Box::new(Broken)));
    let result = call(&mut empty, "search_music", &json!({ "description": "x" }));
    let answer = ok(&result);
    assert_eq!(answer["results"], json!([]));
    assert!(answer["summary"].as_str().is_some_and(|s| s.contains("empty")), "{}", answer["summary"]);
    assert_eq!(answer["tally"]["returned"], 0);
}

#[test]
fn audition_is_unavailable_because_the_server_never_owns_playback() {
    let mut h = host();
    let (code, message, retryable) = failure(&call(&mut h, "audition", &json!({ "asset_id": id_of(1) })));
    assert_eq!((code.as_str(), retryable), ("unavailable", false));
    assert!(message.contains("running app"), "{message}");
    // It is still validated first: a malformed id is the caller's to fix, not an `unavailable`.
    assert_eq!(failure(&call(&mut h, "audition", &json!({ "asset_id": "nope" }))).0, "bad_argument");
}

// ---------------------------------------------------------------------------------------------------------------
// The process
// ---------------------------------------------------------------------------------------------------------------

/// A scratch directory that removes itself.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!("bevy_siren_server_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// An index whose vectors are the fixture model's own output for three phrases, written where the binary reads it.
fn write_tiny_library(dir: &Scratch) {
    let text = std::fs::read_to_string(fixture("clap-tokenizer.json")).expect("the fixture is checked in");
    let vocabulary = Vocabulary::from_json(&text).expect("CLAP's own tokenizer.json parses");
    let mut model = Model::load(&fixture("tiny-text.onnx")).expect("the fixture loads");
    let mut index = Index::new(ModelStamp::measured_clap());
    for (hash, phrase) in [(0xa0u64, "a low rumble"), (0xb0, "white noise hiss"), (0xc0, "a steady tone")] {
        let query = Query::new(phrase).expect("a description");
        let ids: Vec<i64> =
            query.tokenize(&vocabulary).expect("tokenises").into_iter().map(i64::from).collect();
        let embedding = model.embed_text(&ids, &vec![1i64; ids.len()]).expect("embeds");
        assert_eq!(embedding.len(), EMBEDDING_DIM);
        index.insert(hash, embedding).expect("inserts");
    }
    indexer::write(&dir.0, &index).expect("the index is written");
}

/// Run `siren-tools` with `args`, give it `input` on stdin and close it, and return everything it did. **Bounded**: a server
/// that did not exit on end of input would otherwise hang the suite.
fn run(args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_siren-tools"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child.stdin.take().expect("a piped stdin").write_all(input.as_bytes()).expect("stdin takes the input");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("the server exits when stdin closes")
        .expect("its output is collected")
}

fn jsonl(bytes: &[u8]) -> Vec<Value> {
    String::from_utf8(bytes.to_vec())
        .expect("stdout is UTF-8")
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("stdout carried a line that is not JSON ({e}): {line}"))
        })
        .collect()
}

/// **`#28`'s first criterion, as a process**: it starts, lists its tools, answers a real search through a real model graph,
/// and ends when stdin does — with **only messages on stdout**.
#[test]
fn the_binary_lists_tools_and_answers_a_search_over_stdio() {
    let dir = Scratch::new("serve");
    write_tiny_library(&dir);
    let (model, vocab) = (fixture("tiny-text.onnx"), fixture("clap-tokenizer.json"));
    let input = [
        request(
            1,
            "initialize",
            &json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test" } }),
        ),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
        request(2, "tools/list", &json!({})),
        request(
            3,
            "tools/call",
            &json!({ "name": "search_music", "arguments": { "description": "a low rumble", "k": 2 } }),
        ),
        request(
            4,
            "tools/call",
            &json!({ "name": "search_music", "arguments": { "description": "x", "k": 0 } }),
        ),
        String::new(),
    ]
    .join("\n");
    let output = run(
        &[dir.0.to_str().expect("a UTF-8 path"), "--model", model.as_str(), "--vocab", vocab.as_str()],
        &input,
    );
    assert!(output.status.success(), "a closed stdin is a normal end: {:?}", output.status);

    let replies = jsonl(&output.stdout);
    let ids: Vec<&Value> = replies.iter().map(|r| &r["id"]).collect();
    assert_eq!(ids, [&json!(1), &json!(2), &json!(3), &json!(4)], "the notification got no reply");
    assert!(replies.iter().all(|r| r["jsonrpc"] == "2.0"));
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[1]["result"]["tools"].as_array().expect("tools").len(), 6);
    let found = &replies[2]["result"]["structuredContent"]["results"];
    assert_eq!(found[0]["asset_id"], "00000000000000a0", "the phrase the index was built from ranks first");
    assert_eq!(found.as_array().expect("results").len(), 2);
    assert_eq!(replies[3]["result"]["isError"], true);
    assert_eq!(replies[3]["result"]["structuredContent"]["error"]["code"], "bad_argument");

    let log = String::from_utf8_lossy(&output.stderr);
    assert!(
        log.contains("3 assets indexed") && log.contains("search_music is on"),
        "the summary is on stderr: {log}"
    );
}

/// Without a model the process still serves, and says on stderr that `search_music` is off.
#[test]
fn the_binary_serves_the_index_only_tools_without_a_model() {
    let dir = Scratch::new("headless");
    write_tiny_library(&dir);
    let input = [
        request(
            1,
            "tools/call",
            &json!({ "name": "set_anchor", "arguments": { "asset_id": "00000000000000a0" } }),
        ),
        request(2, "tools/call", &json!({ "name": "find_compatible_music", "arguments": { "k": 2 } })),
        request(3, "tools/call", &json!({ "name": "search_music", "arguments": { "description": "x" } })),
    ]
    .join("\n");
    let output = run(&[dir.0.to_str().expect("a UTF-8 path")], &input);
    assert!(output.status.success());
    let replies = jsonl(&output.stdout);
    assert_eq!(replies.len(), 3, "the last line had no newline and is still read");
    assert_eq!(replies[0]["result"]["isError"], false);
    // The anchor's two neighbours are returned or reported as variants — **none silently disappears**. (The fixture model
    // embeds similar phrases similarly enough that the second can be suppressed as a near-duplicate of the first.)
    let answer = &replies[1]["result"]["structuredContent"];
    let returned = answer["results"].as_array().expect("results").len();
    let variants = answer["tally"]["variants"].as_array().expect("variants").len();
    let tally = &answer["tally"];
    let counted = returned
        + variants
        + ["excluded", "below_floor"]
            .map(|k| tally[k].as_u64().expect("a count") as usize)
            .iter()
            .sum::<usize>();
    assert_eq!(counted, 2, "every other asset is returned or accounted for: {answer}");
    assert_eq!(replies[2]["result"]["structuredContent"]["error"]["code"], "unavailable");
    assert!(String::from_utf8_lossy(&output.stderr).contains("search_music is off"));
}

/// **It refuses to start loudly and cleanly**: a non-zero status, the reason on stderr, and **nothing on stdout** — which is
/// the transport's rule, since a client would read anything there as a message.
#[test]
fn the_binary_refuses_to_start_without_an_index_and_prints_nothing_on_stdout() {
    let output = run(&["/nonexistent/bevy_siren/library"], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "stdout carries only messages: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(log.contains(indexer::INDEX_FILE) && log.contains("cannot use the index"), "{log}");

    let dir = Scratch::new("model");
    write_tiny_library(&dir);
    let path = dir.0.to_str().expect("a UTF-8 path");
    let output =
        run(&[path, "--model", "/nonexistent/m.onnx", "--vocab", &fixture("clap-tokenizer.json")], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot load the model"));
}

/// **An index from another export is refused until its version is given.** The measured version is only the default: before
/// `--model-version`, `siren-tools` could serve exactly one torch and one transformers — the author's — and the first real
/// export on another machine (`transformers-4.57.6/torch-2.8.0/opset-17`) was a stamp mismatch with no way past it.
#[test]
fn an_index_from_another_export_is_served_once_its_version_is_given() {
    let dir = Scratch::new("version");
    let mut index = Index::new(ModelStamp::clap("my-export/1"));
    for hash in [0xa0u64, 0xb0] {
        index.insert(hash, vec![hash as f32; EMBEDDING_DIM]).expect("inserts");
    }
    indexer::write(&dir.0, &index).expect("the index is written");
    let path = dir.0.to_str().expect("a UTF-8 path");

    // Without the flag: refused, naming **both** versions, with nothing on stdout.
    let refused = run(&[path], "");
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    let log = String::from_utf8_lossy(&refused.stderr);
    assert!(
        log.contains("my-export/1") && log.contains(bevy_siren::semantic::MEASURED_CLAP_VERSION),
        "{log}"
    );

    // With it: served, and the tools that need only the index work.
    let input = request(
        1,
        "tools/call",
        &json!({ "name": "set_anchor", "arguments": { "asset_id": "00000000000000a0" } }),
    );
    let served = run(&[path, "--model-version", "my-export/1"], &input);
    assert!(served.status.success(), "{:?}", String::from_utf8_lossy(&served.stderr));
    let replies = jsonl(&served.stdout);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["result"]["isError"], false);
    assert!(String::from_utf8_lossy(&served.stderr).contains("2 assets indexed"));

    // A version that is not the one the index names is the same refusal in the other direction.
    let wrong = run(&[path, "--model-version", "some-other-export"], "");
    assert_eq!(wrong.status.code(), Some(1));
    assert!(wrong.stdout.is_empty());

    // And the usage says the flag exists and what its default is, so a person finds it from the refusal.
    let usage = String::from_utf8_lossy(&run(&[], "").stderr).into_owned();
    assert!(
        usage.contains("--model-version") && usage.contains(bevy_siren::semantic::MEASURED_CLAP_VERSION),
        "{usage}"
    );
}

#[test]
fn the_binary_says_what_it_takes_when_misused() {
    for (args, needle) in [
        (vec![], "a library directory is required"),
        (vec!["a", "b"], "one library directory only"),
        (vec!["a", "--model", "m.onnx"], "--model and --vocab go together"),
        (vec!["a", "--vocab", "v.json"], "--model and --vocab go together"),
        (vec!["a", "--frobnicate"], "unknown option"),
        (vec!["a", "--model"], "--model needs a file"),
        (vec!["a", "--model-version"], "--model-version needs a version string"),
    ] {
        let output = run(&args, "");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let log = String::from_utf8_lossy(&output.stderr);
        assert!(log.contains(needle) && log.contains("usage:"), "{args:?}: {log}");
    }
}

/// **`#28`'s fourth criterion**: `cargo build` without the feature builds no binary. `required-features` is what does it, and
/// removing that line would put a server that cannot link into every consumer's build.
#[test]
fn the_binary_is_built_only_behind_the_semantic_feature() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).expect("manifest");
    let section = manifest
        .split("\n[[bin]]\n")
        .nth(1)
        .expect("the manifest declares the tool server")
        .split("\n[")
        .next()
        .expect("a section");
    assert!(section.contains("name = \"siren-tools\""), "{section}");
    assert!(section.contains("required-features = [\"semantic\"]"), "{section}");
}
