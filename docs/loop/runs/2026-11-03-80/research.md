# Iteration 80 — research: two ratchets, one decision

## Sources

| Source | What it states |
|---|---|
| `tests/leaf.rs::the_dependency_list_stays_closed` | `ALLOWED_DEPS = ["bevy","serde","ron","rodio","ort","rubato","ebur128"]`, and *"Adding a name here is a design decision, not a convenience: the sentence above is the argument, and it belongs in the same commit as the entry."* |
| `tests/library.rs::the_semantic_feature_declares_no_dependency_of_its_own` | the **manifest's** `semantic = [...]` line must equal a pinned string, and *"Anything more is a design decision: add it to ALLOWED_DEPS in tests/leaf.rs with the sentence that argues for it, in the same commit."* |
| this crate, `Cargo.toml` | `semantic = ["dep:ort", "dep:rodio", "dep:rubato", "dep:ebur128"]` |
| this crate, `git log` | `63ba157` (iteration 77) added `ebur128` and updated `tests/leaf.rs` only |

## The technique, in the sources' terms

The crate guards its dependency list **twice, from opposite directions**:

1. **`tests/leaf.rs`** reads `Cargo.toml`'s `[dependencies]` table and asserts every **declared** crate is in a list of
   allowed names. It answers *"may this crate exist here?"*
2. **`tests/library.rs`** reads the `semantic = [...]` **feature value** and asserts it names exactly three (now four)
   dependencies. It answers *"does the feature pull anything beyond the ones we decided?"*

The first is a list; the second is a pinned string. A new dependency that is added to `[dependencies]` and to the
feature, but not to the list, fails (1). One added to `[dependencies]` and the list but **not** to the pinned string
fails (2) — which is what happened.

Both messages are unusually good: each names the other file and says *"in the same commit"*. That is why this is a
five-minute fix rather than an archaeology exercise.

## Parameters the sources report

| Parameter | Value |
|---|---|
| dependencies the semantic feature may name | **4**, each with an argument in `tests/leaf.rs` |
| the pinned string | `["dep:ort", "dep:rodio", "dep:rubato", "dep:ebur128"]` |
| argument for `ebur128` | pure Rust with `default-features = false` (no `cc`, no C object, 1.66 s build); agrees with `ffmpeg`'s reference to **0.01 LU** — ADR 0003 |

## What good sounds like

A dependency change that is visible in review **at the feature line as well as the table**, so a reviewer sees exactly
what enabling `semantic` costs. The two guards are not redundancy: one is a policy list a human reads, the other is a
machine-checked value that cannot drift from the manifest.

The failure mode this iteration fixes is the reason both exist — and the reason the loop's own rule is *"re-run the
whole baseline; a case that went red is this iteration's failure."* The disk made that rule unenforceable for five
iterations, and the debt came due.
