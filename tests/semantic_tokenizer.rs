//! **The tokeniser, pinned against CLAP's own.** Every expected id below was produced by
//! `ClapProcessor.from_pretrained("laion/clap-htsat-fused").tokenizer.encode(text, add_special_tokens=False)` on
//! 2026-11-03 and is recorded in `docs/loop/runs/2026-11-03-66/evidence/ids.json`.
//!
//! A tokeniser that is *nearly* right is worse than none: the wrong ids produce a plausible embedding from the wrong
//! words, and nothing downstream fails. So the test is exact equality on ten strings chosen to cover the cases that
//! break naive implementations — a word that splits into two pieces, a word that does not split, punctuation, a
//! multi-word phrase, and a comma-separated list.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::tokenizer::{TokenProblem, Vocabulary};

fn vocab() -> Vocabulary {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/clap-tokenizer.json");
    let text = std::fs::read_to_string(path).expect("the fixture is checked in");
    Vocabulary::from_json(&text).expect("CLAP's own tokenizer.json parses")
}

#[test]
fn the_vocabulary_is_the_models_own() {
    let v = vocab();
    assert_eq!(v.len(), 50_265, "RoBERTa's vocabulary size");
    assert_eq!(v.merges(), 50_000, "and its merge table");
    assert!(!v.is_empty());
}

#[test]
fn ten_strings_match_the_reference_ids_exactly() {
    let v = vocab();
    // (text, ids from transformers 5.22.1's ClapProcessor, add_special_tokens=False)
    let cases: &[(&str, &[u32])] = &[
        ("tense", &[90, 9401]),
        ("underwater", &[5087, 5412]),
        ("ruins", &[2070, 1344]),
        ("rain", &[9946]),
        ("falling", &[7349, 154]),
        ("a warm footstep on gravel", &[102, 3279, 2767, 13975, 15, 26923]),
        ("tense, sparse, underwater ruins", &[90, 9401, 6, 28593, 6, 16974, 24757]),
        ("white noise hiss", &[9830, 6496, 39, 29]),
        ("a low rumble", &[102, 614, 11122, 5225]),
        ("a steady musical tone", &[102, 5204, 4388, 6328]),
    ];
    for (text, expected) in cases {
        let got = v.encode(text).unwrap_or_else(|e| panic!("`{text}`: {e}"));
        assert_eq!(&got[..], *expected, "`{text}` must tokenise to the model's own ids");
    }
}

#[test]
fn a_word_that_splits_is_not_looked_up_whole() {
    // The failure a dictionary would have: `tense` is in the vocabulary as one token only if the merges run.
    let v = vocab();
    let ids = v.encode("tense").expect("encodes");
    assert_eq!(ids.len(), 2, "`tense` is two pieces, not one: {ids:?}");
    assert_ne!(ids.len(), 1, "a word-level lookup would produce one id and be wrong");
}

#[test]
fn the_specials_are_the_roberta_pair() {
    let v = vocab();
    let with = v.encode_for_model("tense").expect("encodes");
    assert_eq!(with.first(), Some(&0), "`<s>` is id 0 in RoBERTa's vocabulary");
    assert_eq!(with.last(), Some(&2), "and `</s>` is id 2");
    assert_eq!(&with[1..with.len() - 1], &[90, 9401], "with the text in between, unchanged");
}

#[test]
fn the_attention_mask_attends_every_token_there_is() {
    let v = vocab();
    let ids = v.encode_for_model("a low rumble").expect("encodes");
    let mask = v.attention_mask(&ids);
    assert_eq!(mask.len(), ids.len());
    assert!(mask.iter().all(|m| *m == 1), "nothing is padded, so nothing is masked: {mask:?}");
}

#[test]
fn an_empty_description_encodes_to_no_ids_and_the_specials_alone() {
    let v = vocab();
    assert_eq!(v.encode("").expect("empty is not an error"), Vec::<u32>::new());
    assert_eq!(v.encode_for_model("").expect("encodes"), vec![0, 2], "just `<s>` and `</s>`");
}

#[test]
fn a_file_that_is_not_a_byte_level_bpe_vocabulary_is_refused() {
    // A word-level vocabulary would produce plausible ids for most words and nothing would fail, so the type is
    // checked rather than assumed.
    let err =
        Vocabulary::from_json("{\"model\": {\"type\": \"WordPiece\", \"vocab\": {}}}").expect_err("refused");
    assert!(matches!(err, TokenProblem::Vocabulary { .. }), "got {err:?}");
    assert!(err.to_string().contains("not a byte-level BPE"), "{err}");

    let err = Vocabulary::from_json("{}").expect_err("no model");
    assert!(matches!(err, TokenProblem::Vocabulary { .. }));
}

#[test]
fn a_vocabulary_with_a_colon_in_a_token_parses() {
    // `":"` and `","` are real tokens, written `"\\":\\": 6` in the file. Splitting on the first colon would take `"`
    // as the key and fail — which is exactly what happened while writing this.
    let v = vocab();
    // If those keys parsed, punctuation encodes to a single id each. Ids checked against the model:
    // `:` is 35, `,` is 6, `.` is 4.
    assert_eq!(v.encode(":").expect("a colon is a token"), vec![35]);
    assert_eq!(v.encode(",").expect("a comma is a token"), vec![6]);
    assert_eq!(v.encode(".").expect("a full stop is a token"), vec![4]);
}

#[test]
fn a_description_becomes_the_ids_the_model_takes() {
    // The whole seam: a validated Query, CLAP's vocabulary, and the ids the text tower runs on.
    use bevy_siren::semantic::query::Query;
    let v = vocab();
    let q = Query::new("  a low rumble  ").expect("a description");
    let ids: Vec<i64> = q.tokenize(&v).expect("tokenises").into_iter().map(i64::from).collect();
    assert_eq!(ids, vec![0, 102, 614, 11122, 5225, 2], "trimmed, specials included, matching the reference");
}

#[test]
fn a_description_longer_than_the_model_accepts_is_refused_by_count() {
    use bevy_siren::semantic::MAX_TEXT_TOKENS;
    use bevy_siren::semantic::query::{Query, QueryProblem};
    let v = vocab();
    // Well past 77 tokens: a sentence repeated until the count exceeds the limit.
    let long = "a low rumble in the dark ".repeat(30);
    let q = Query::new(&long).expect("valid text");
    let err = q.tokenize(&v).expect_err("too long");
    match err {
        QueryProblem::TooLong { need, limit } => {
            assert!(need > MAX_TEXT_TOKENS, "{need} > {MAX_TEXT_TOKENS}");
            assert_eq!(limit, MAX_TEXT_TOKENS);
        }
        other => panic!("expected TooLong, got {other:?}"),
    }
    assert!(err.to_string().contains(&MAX_TEXT_TOKENS.to_string()), "{err}");
}

#[test]
fn the_token_count_excludes_the_specials() {
    // `<s>` and `</s>` are the model's, not the description's, so they must not eat the budget.
    use bevy_siren::semantic::query::Query;
    let v = vocab();
    let q = Query::new("tense").expect("a description");
    let ids = q.tokenize(&v).expect("tokenises");
    assert_eq!(ids.len(), 4, "<s> + two pieces + </s>");
    // Two body tokens, which is what `check_tokens` sees.
    assert!(q.check_tokens(ids.len() - 2).is_ok());
}
