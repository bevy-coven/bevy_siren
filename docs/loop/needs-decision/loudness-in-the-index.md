# Where per-asset loudness lives in the index

- **Issue:** [#21](https://github.com/bevy-coven/bevy_siren/issues/21), criterion *"both values are visible per asset
  and survive an index round trip"*
- **Status:** recommends, does not decide

## The question

`semantic::loudness` measures an asset's integrated loudness and true peak. **Where does the number get stored** so a
later query can filter on it, and in what format?

## What exists today

`Index` (`src/semantic/index.rs`) is a **versioned text format**:

```
bevy_siren index 1
count <n>
checksum <16 hex>
<hash> <dim floats...>
```

`MAGIC` is the literal string `"bevy_siren index 1"`, and `Index::load` rejects a file whose header does not match — so
any change to the record layout is a **version bump plus a migration story**, not a silent append.

## Options

### A. A fourth field on the record line: `<hash> <dim floats...> <lufs> <dbtp>`

- **Cost:** every existing index fails to load until rewritten. The MAGIC string must become `bevy_siren index 2`, and a
  user who upgrades the crate loses their index once.
- **Benefit:** one file, one parse, and `to_text` stays trivially checkable with a checksum over the body.
- **Risk:** `None` (too short to measure) has no natural textual spelling beside a float. It needs a sentinel — `-inf` or
  a literal `-` — and a sentinel in a numeric column is how the −70 trap gets recreated in a new place.

### B. A separate `loudness` section after the vectors, keyed by hash

- **Cost:** a second pass over the file on load, and a second count/checksum to keep honest.
- **Benefit:** **backward compatible in the direction that matters** — an index with no loudness section still loads, and
  an asset with no measurement is simply absent from the section rather than carrying a sentinel. `None` is represented
  by *absence*, which is the same distinction `Loudness` already draws with `Option`.
- **Benefit:** loudness can be re-measured and rewritten **without touching the vectors**, which is exactly what
  happens when BS.1770 is re-run after a decoder fix.

### C. A sibling sidecar file, `<index>.loudness`

- **Cost:** two files a user can lose independently, and a checksum that cannot cover both.
- **Benefit:** the index format does not move at all.
- **Risk:** a caller must now handle "index present, loudness missing", which is the same state option B represents
  in-file but without the atomic write.

## Recommendation

**Option B**, and the reason is the sentinel: `Loudness::integrated` is an `Option` because a 0.2 s footstep has no
integrated loudness, and *the whole point of ADR 0003's short-clip finding* is that this absence must never be encoded
as a number. Option A forces a sentinel into a numeric column; option B encodes absence as absence. It also lets a
project re-measure loudness without rewriting a 512-dimensional vector per asset.

The format gains:

```
bevy_siren index 1
count <n>
checksum <16 hex>
<hash> <dim floats...>
...
loudness <m>
<16 hex hash> <lufs or -> <dbtp>
...
```

with the section optional: a file that ends after the vectors loads exactly as it does today.

## What each choice costs a consumer

| | A | B | C |
|---|---|---|---|
| existing index keeps working | **no** | **yes** | yes |
| an asset with no measurement | sentinel | absent | absent |
| re-measure without rewriting vectors | no | **yes** | yes |
| one file to keep | yes | yes | **no** |

## Why this is not decided here

It changes the **definitions-file schema** (`MAGIC`'s version and the record layout), and this crate's rule is that such a
change goes to a decision file rather than being made inline. The measurement itself — `semantic::loudness` — is
independent of where the number is stored, and it is built and tested now.
