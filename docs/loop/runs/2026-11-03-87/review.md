# Iteration 87 — review

## Pass 1 — Function: is the 3× claim now true and asserted?

**Critique.** The claim has been **quoted in four iterations' logs** and asserted by nothing, which is the definition of a
magic constant. It is now a test:

| | bytes | per clip |
|---|---|---|
| binary, 5 000 clips | **10 280 000 (10.28 MB)** | 2 056 = `u64` + 512 × `f32` |
| text, 5 000 clips | **31.88 MB** | 6 376.4 |
| ratio | **≈3.1×** | ADR 0004 said 3× |

**And it round-trips.** `the_binary_form_round_trips_what_the_text_form_does` asserts that the two formats carry the
**same index** — vectors bit for bit, provenance including a pack name with a space, and loudness including an **absent**
integrated value. That last one is the load-bearing assertion: a presence byte is the whole reason the loudness section is
not a bare `f32`.

**Verdict: GREAT.**

## Pass 2 — Zoom: the fixture that made the claim look false

**Critique.** This is the pass that matters, because the failure was **in the test**, and it is instructive.

The first size test built 5 000 records of `vec![0.5f32; 512]`. `{:?}` on `0.5` is `0.5` — **one character** — so the text
form produced:

    1.00× the binary form  (10 325 169 vs 10 280 115 bytes)

The claim looked **false**. It is not: a real embedding is 512 unrelated values near zero, each printing eight or nine
characters. Recomputed on a deterministic spread, the text form is **6 376.4 bytes per record** and 5 000 clips come to
**31.88 MB** — exactly ADR 0004's figure.

**The general lesson, worth the log**: a fixture chosen for convenience can make a *true* claim fail, and a test that
fails for a fixture reason is as misleading as one that passes for one. The fix is not to loosen the bound but to make
the fixture resemble the data.

**A second, smaller find**: the refusal test used `b"\x00\x01 not an index"`, which is **valid UTF-8**, so it took the
text branch and got *"unreadable index header"* rather than the *"neither a binary index nor UTF-8"* message the test
expected. Both paths are now covered with inputs that reach them, and the specific message is noted as the better of the
two — a refusal that points at the bytes beats one that only says "wrong".

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what changes for a consumer?

**Critique.** Nothing required. `build` writes a smaller file under the same call, and `read` loads **both** formats,
because it decides from the first four bytes rather than the file name:

```rust
if bytes.starts_with(MAGIC_BINARY) { Index::from_bytes(..) } else { Index::load(utf8(..)) }
```

So a project that built an index yesterday finds it today. The file is now called `siren-index` rather than
`siren-index.txt`, and the name carries no claim — the bytes do.

**A truncation in any section is caught by one rule** — the checksum over every preceding byte — so a half-written file
says *"the index body does not match its checksum"* rather than failing later on a length that happens to be wrong.

**The honest gap**: `f16` stays declined (ADR 0004's reason — `rank::top_k` compares against a floor), and `#36`'s
throughput number is still untaken. Neither is this iteration's.

**Verdict: GREAT.**
