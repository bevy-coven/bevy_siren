//! The README's quick start is the crate-root doctest, so the copy a reader pastes is the copy that compiles.
//! Run with `cargo test --features readme-example --test all readme::`.
#![cfg(feature = "readme-example")]

const README: &str = include_str!("../README.md");
const LIB: &str = include_str!("../src/lib.rs");

/// The ```` ```rust ```` block under `## Quick start` in the README. The fence's marker (`rust` or
/// `rust,no_run`) is not compared: the README's block is `no_run` because a reader must not have a window opened
/// by a doc test, and the crate doc's is `no_run` for the same reason.
fn readme_block() -> Vec<&'static str> {
    let section = README.split("\n## Quick start").nth(1).expect("README.md has a `## Quick start` section");
    let body: Vec<&str> = section
        .lines()
        .skip_while(|l| !l.starts_with("```rust"))
        .skip(1)
        .take_while(|l| *l != "```")
        .collect();
    assert!(!body.is_empty(), "the quick start has a ```rust block");
    body
}

/// The `no_run` block in the crate doc, without its `//! ` prefixes or `# ` hidden lines.
fn doctest_block() -> Vec<&'static str> {
    let doc: Vec<&str> =
        LIB.lines().filter_map(|l| l.strip_prefix("//!")).map(|l| l.strip_prefix(' ').unwrap_or(l)).collect();
    doc.iter()
        .skip_while(|l| **l != "```no_run")
        .skip(1)
        .take_while(|l| **l != "```")
        .filter(|l| !(l.starts_with("# ") || **l == "#"))
        .copied()
        .collect()
}

#[test]
fn the_readme_quick_start_is_the_doctest() {
    let (readme, doctest) = (readme_block(), doctest_block());
    assert!(!doctest.is_empty(), "src/lib.rs has a ```no_run block");
    assert_eq!(readme, doctest, "README.md's quick start differs from the crate-root doctest in src/lib.rs");
}
