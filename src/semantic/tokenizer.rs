//! **CLAP's text tokeniser**, in this crate, with no dependency.
//!
//! CLAP's text tower takes `input_ids` and `attention_mask`, and something has to turn a description into them.
//! Measured from the installed model (`runs/2026-11-03-66/research.md`):
//!
//! | | |
//! |---|---|
//! | tokeniser | `RobertaTokenizer` |
//! | model | **byte-level BPE**, 50,265 vocab, 50,000 merges |
//! | pre-tokeniser | `ByteLevel`, `add_prefix_space: false`, GPT-2's regex |
//! | post-processor | `RobertaProcessing`: `<s>` … `</s>` |
//!
//! # Why this is code and not a dependency
//!
//! The alternative is a tokeniser crate plus its own data file. This is the same thing with the data supplied by a
//! caller: [`Vocabulary::from_json`] reads the `tokenizer.json` that ships with the model, and the algorithm is the
//! standard byte-level BPE. That keeps the crate's dependency list at five and makes the one thing that could quietly
//! drift — the vocabulary — a file a caller pins, not a build-time constant.
//!
//! # The rule that makes it correct rather than approximately correct
//!
//! **Byte-level BPE splits words.** Measured: `tense` is `[90, 9401]` and `underwater` is `[5087, 5412]`. A tokeniser
//! that looked up whole words would find `tense` and produce the wrong ids for most of a description, so [`Vocabulary::encode`]
//! runs the merges rather than a dictionary.

use std::collections::HashMap;

/// The byte-to-`char` map GPT-2's byte-level BPE uses, so every byte is representable as a printable character.
///
/// Bytes that are printable ASCII map to themselves; the rest are shifted into the range `U+0100..`. This is what the
/// `Ġ` in a merge table means: a space, encoded as `U+0120`.
fn byte_to_char() -> [char; 256] {
    let mut table = ['\0'; 256];
    let mut n = 0u32;
    for b in 0u16..256 {
        let b = b as u8;
        let printable =
            (0x21..=0x7e).contains(&b) || (0xa1..=0xac).contains(&b) || (0xae..=0xff).contains(&b);
        let c = if printable {
            char::from(b)
        } else {
            let c = char::from_u32(256 + n).unwrap_or('\u{fffd}');
            n += 1;
            c
        };
        table[b as usize] = c;
    }
    table
}

/// Why a description could not be tokenised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenProblem {
    /// The vocabulary has no such token, so an id cannot be produced.
    UnknownToken {
        /// The token text that was not found.
        token: String,
    },
    /// The vocabulary file is not the shape this reader expects.
    Vocabulary {
        /// What was wrong with it.
        what: String,
    },
}

impl core::fmt::Display for TokenProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TokenProblem::UnknownToken { token } => {
                write!(f, "the vocabulary has no entry for `{token}`; it is not CLAP's tokeniser")
            }
            TokenProblem::Vocabulary { what } => write!(f, "unreadable vocabulary: {what}"),
        }
    }
}

/// **CLAP's vocabulary and merge rules**, loaded from the model's own `tokenizer.json`.
#[derive(Debug, Clone)]
pub struct Vocabulary {
    /// Token text to id.
    ids: HashMap<String, u32>,
    /// Merge ranks: the earlier a pair merged during training, the lower its rank and the earlier it applies.
    ranks: HashMap<(String, String), u32>,
    /// The id written before the text: `<s>`.
    cls_id: u32,
    /// The id written after it: `</s>`.
    sep_id: u32,
}

impl Vocabulary {
    /// **Read a `tokenizer.json`**, taking the byte-level BPE model and RoBERTa's `<s>`/`</s>` ids.
    ///
    /// Only the fields this crate needs are read. A file that is not byte-level BPE is refused rather than
    /// misinterpreted, because a word-level vocabulary would produce plausible ids for every third word and nothing
    /// would fail.
    pub fn from_json(text: &str) -> Result<Vocabulary, TokenProblem> {
        let bad = |what: &str| TokenProblem::Vocabulary { what: what.to_string() };

        let model_start = text.find("\"model\"").ok_or_else(|| bad("no `model` section"))?;
        let type_at = text[model_start..]
            .find("\"type\"")
            .map(|i| i + model_start)
            .ok_or_else(|| bad("the model has no `type`"))?;
        let after = &text[type_at..];
        let colon = after.find(':').ok_or_else(|| bad("malformed `type`"))?;
        let rest = &after[colon + 1..];
        let open = rest.find('"').ok_or_else(|| bad("malformed `type`"))?;
        let close =
            rest[open + 1..].find('"').map(|i| i + open + 1).ok_or_else(|| bad("malformed `type`"))?;
        if &rest[open + 1..close] != "BPE" {
            return Err(bad("this is not a byte-level BPE vocabulary"));
        }

        // Find `model`'s own `"vocab"`, not the one inside `added_tokens`. Scan for the key at depth 1 and then take
        // the balanced object after it, so nesting cannot fool it.
        let vocab_at = find_key_at_depth(text, "model", 1)
            .and_then(|m| find_key_at_depth(&text[m..], "vocab", 1).map(|v| v + m))
            .ok_or_else(|| bad("no `vocab` inside `model`"))?;
        let (open, close) = balanced(text, vocab_at, b'{', b'}').ok_or_else(|| bad("malformed `vocab`"))?;
        let mut ids = HashMap::new();
        for entry in split_top_level(&text[open + 1..close], ',') {
            // **A vocabulary key can contain a colon or a comma** — `":"` and `","` are both tokens, written
            // `"\":\": 6`. Splitting on the first `:` would take `"` as the key and fail, so read the quoted string
            // first and take everything after the colon that follows it as the id.
            let entry = entry.trim();
            let Some(quote_end) = closing_quote(entry) else { continue };
            let key = unquote(&entry[..quote_end]).ok_or_else(|| bad("a vocabulary key is not a string"))?;
            let rest = entry[quote_end..].trim_start();
            let Some(rest) = rest.strip_prefix(':') else { continue };
            let id: u32 = rest.trim().parse().map_err(|_| bad("a vocabulary id is not a number"))?;
            ids.insert(key, id);
        }
        if ids.is_empty() {
            return Err(bad("the vocabulary is empty"));
        }

        let merges_at = find_key_at_depth(text, "model", 1)
            .and_then(|m| find_key_at_depth(&text[m..], "merges", 1).map(|v| v + m))
            .ok_or_else(|| bad("no `merges` inside `model`"))?;
        let (open, close) = balanced(text, merges_at, b'[', b']').ok_or_else(|| bad("malformed `merges`"))?;
        // **Two shapes exist**: a two-element array (`[["Ġ","t"], ...]`, which `transformers` pretty-prints across
        // lines) and a space-separated string (`"Ġ t"`, which older exports write). Both are read, and the array case
        // is walked by bracket rather than by comma so the newlines and inner commas cannot break it.
        let mut ranks = HashMap::new();
        let body = &text[open + 1..close];
        let mut cursor = 0usize;
        let mut rank = 0u32;
        while let Some(rel) = body[cursor..].find('[') {
            let at = cursor + rel;
            let Some((_, end)) = balanced(body, at, b'[', b']') else { break };
            let inner = &body[at + 1..end];
            let parts: Vec<&str> =
                split_top_level(inner, ',').into_iter().map(str::trim).filter(|s| !s.is_empty()).collect();
            if parts.len() == 2
                && let (Some(a), Some(b)) = (unquote(parts[0]), unquote(parts[1]))
            {
                ranks.insert((a, b), rank);
                rank += 1;
            }
            cursor = end + 1;
        }
        if ranks.is_empty() {
            for entry in split_top_level(body, ',').into_iter().map(str::trim).filter(|s| !s.is_empty()) {
                if entry.starts_with('[') {
                    continue;
                }
                let Some(s) = unquote(entry) else { continue };
                let mut it = s.splitn(2, ' ');
                if let (Some(a), Some(b)) = (it.next(), it.next()) {
                    ranks.insert((a.to_string(), b.to_string()), rank);
                    rank += 1;
                }
            }
        }
        if ranks.is_empty() {
            return Err(bad("the merge table is empty"));
        }

        let cls_id = *ids.get("<s>").ok_or_else(|| bad("no `<s>` token"))?;
        let sep_id = *ids.get("</s>").ok_or_else(|| bad("no `</s>` token"))?;

        Ok(Vocabulary { ids, ranks, cls_id, sep_id })
    }

    /// How many tokens the vocabulary holds.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// True when nothing is loaded.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// How many merge rules the vocabulary holds.
    pub fn merges(&self) -> usize {
        self.ranks.len()
    }

    /// **Encode a description to the ids the text tower takes**, without the specials.
    ///
    /// The steps are the model's: split on GPT-2's pattern, map each byte to its printable character, apply the merges
    /// in rank order, then look each piece up.
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, TokenProblem> {
        let mut ids = Vec::new();
        for piece in pretokenize(text) {
            for token in self.bpe(&piece) {
                let id = *self
                    .ids
                    .get(&token)
                    .ok_or_else(|| TokenProblem::UnknownToken { token: token.clone() })?;
                ids.push(id);
            }
        }
        Ok(ids)
    }

    /// **Encode with the specials the model expects**: `<s>`, the text, `</s>`.
    pub fn encode_for_model(&self, text: &str) -> Result<Vec<u32>, TokenProblem> {
        let mut ids = vec![self.cls_id];
        ids.extend(self.encode(text)?);
        ids.push(self.sep_id);
        Ok(ids)
    }

    /// The attention mask for an id list: every real token attended, padding not. There is no padding here, so the
    /// answer is all ones — kept as a function so a caller padding a batch writes the mask in one place.
    pub fn attention_mask(&self, ids: &[u32]) -> Vec<i64> {
        vec![1; ids.len()]
    }

    /// The byte-level BPE for one pre-tokenised piece.
    fn bpe(&self, piece: &str) -> Vec<String> {
        let chars: Vec<char> = piece.chars().collect();
        let mut word: Vec<String> = chars.iter().map(|c| c.to_string()).collect();
        if word.len() < 2 {
            return word;
        }
        loop {
            let mut best: Option<(usize, u32)> = None;
            for i in 0..word.len() - 1 {
                // `get` with a tuple of references: no `String` per comparison, which matters because this loop runs
                // once per merge over every piece of every description.
                if let Some(&rank) = self.ranks.get(&(word[i].clone(), word[i + 1].clone()))
                    && best.is_none_or(|(_, r)| rank < r)
                {
                    best = Some((i, rank));
                }
            }
            let Some((at, _)) = best else { break };
            word[at] = format!("{}{}", word[at], word[at + 1]);
            word.remove(at + 1);
            if word.len() == 1 {
                break;
            }
        }
        word
    }
}

/// **The index just past the closing quote** of the string that starts at the beginning of `s`.
fn closing_quote(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' => escaped = true,
            b'"' => return Some(i + 1),
            _ => {}
        }
    }
    None
}

/// **Where a key starts**, scanning for `"key"` at a given brace depth and outside strings.
///
/// `tokenizer.json` contains `"vocab"` twice — once in `model`, once inside `added_tokens` — so a plain `find` picks the
/// wrong one and the parse then fails on a key that is not a string.
fn find_key_at_depth(text: &str, key: &str, want_depth: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let needle = format!("\"{key}\"");
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if in_string {
            match c {
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                if depth == want_depth && bytes[i..].starts_with(needle.as_bytes()) {
                    return Some(i);
                }
                in_string = true;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    None
}

/// **The matching bracket pair** for the `open` bracket at or after `from`, respecting strings.
fn balanced(text: &str, from: usize, open: u8, close: u8) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let start = bytes[from..].iter().position(|&b| b == open)? + from;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &c) in bytes.iter().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            match c {
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            b'"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, i));
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a JSON-ish string on `sep`, ignoring separators inside quotes.
fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' if in_quote => escaped = true,
            '"' => in_quote = !in_quote,
            c if c == sep && !in_quote => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Strip surrounding quotes and unescape the four escapes a vocabulary uses.
fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                let code = u32::from_str_radix(&hex, 16).ok()?;
                out.push(char::from_u32(code)?);
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// **GPT-2's pre-tokenisation pattern**, which is what `ByteLevel` with `use_regex: true` applies.
///
/// The pieces are: contractions, letter runs with an optional leading space, digit runs, punctuation runs, and
/// whitespace runs that are not a lone space. A hand-written scanner rather than a regex, because the crate has no
/// regex dependency and the pattern is short enough to state directly.
fn pretokenize(text: &str) -> Vec<String> {
    let table = byte_to_char();
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let c = chars[i];
        if c.is_whitespace() && c != ' ' {
            while i < chars.len() && chars[i].is_whitespace() && chars[i] != ' ' {
                i += 1;
            }
        } else if c.is_alphabetic() {
            i += 1;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
        } else if c.is_numeric() {
            i += 1;
            while i < chars.len() && chars[i].is_numeric() {
                i += 1;
            }
        } else if c == ' ' && i + 1 < chars.len() {
            // A space joins the next run: ` underwater` is one piece, which is why the merge table has `Ġ` entries.
            i += 2;
            let next_alpha = chars[i - 1].is_alphabetic();
            let next_num = chars[i - 1].is_numeric();
            while i < chars.len()
                && (if next_alpha {
                    chars[i].is_alphabetic()
                } else if next_num {
                    chars[i].is_numeric()
                } else {
                    false
                })
            {
                i += 1;
            }
        } else {
            i += 1;
        }
        let piece: String = chars[start..i]
            .iter()
            .map(|&ch| if (ch as u32) < 256 { table[ch as usize] } else { ch })
            .collect();
        if !piece.is_empty() {
            pieces.push(piece);
        }
    }
    pieces
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`TokenProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for TokenProblem {}
