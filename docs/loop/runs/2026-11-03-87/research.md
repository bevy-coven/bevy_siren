# Iteration 87 — research: the binary index, and the fixture that made the 3× claim look false

The item is ADR 0004's remaining work: the binary writer, whose **3× overhead has been quoted in four iterations' logs
and never asserted by a test**.

## Sources

| Source | The rule it reports |
|---|---|
| `docs/adr/0004-vector-store.md` | binary `f32` is **10.30 MB** at 5 000 clips against the text form's **31.88 MB**; the file identifies itself by magic, not by extension |
| `src/semantic/index.rs::to_text` | each float is written with `{:?}` — Rust's **shortest round-trip** form |
| BS.1770 via ADR 0003 | a short clip has **no** integrated loudness, so a storage byte must say *present or absent* |
| `#14`'s decision file | absence is encoded as absence; a sentinel number is how `−70 LUFS` became a lie |

## The technique, in the sources' terms

A vector is 512 `f32`s. As text each is eight or nine characters; as bytes each is exactly four. The binary layout is
therefore a header, then `hash ++ 512 × f32` per record, then the two optional sections, then a checksum over
everything before it. Little-endian throughout, because a file that a game ships is read on one machine and there is no
cross-architecture exchange to design for.

Three details are forced:

1. **The checksum covers every byte**, including both trailing sections. A truncated file must be detected by the same
   rule whatever section it was cut in.
2. **A presence byte for the integrated loudness.** Writing a sentinel there would recreate the exact defect ADR 0003
   measured three implementations making; `0` means *no measurement* and `1` means *the following `f32` is one*.
3. **A distinct magic**, `BSI2`, so a reader tells the format from the first four bytes. `read` uses that rather than the
   file name, which keeps an index written as text by an older build loading unchanged.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| binary magic | `BSI2` | this design |
| text magic | `bevy_siren index 1` | existing |
| record | `u64` hash + 512 × `f32` = **2 056 bytes** | arithmetic |
| 5 000 clips, binary | **10 280 000 B ≈ 10.28 MB** | `5 000 × 2 056` |
| 5 000 clips, text | **31.88 MB** | ADR 0004, reproduced at **6 376.4 bytes/record** |
| text ÷ binary | **≈ 3.1** | measured on realistic values |

## What good sounds like — and the trap this iteration fell into

A 5 000-clip index is **10.28 MB** as bytes and about **31.88 MB** as text, and both formats load to the **same index**,
including a pack name with a space and an absent integrated loudness.

**The trap**: the first version of the size test built 5 000 records of `vec![0.5f32; 512]` — a constant. A constant
prints as `0.5`, **one character**, so the text form came out the same size as the binary form and the test failed with
`the text form is ~3x the binary form: got 1.00`. The 3× claim is true for a **real embedding**, which is 512 unrelated
values near zero; a fixture of repeated constants tests nothing about float formatting. Recomputed on a deterministic
spread: **6 376.4 bytes per text record**, 31.88 MB for 5 000 — matching ADR 0004 to the byte.
