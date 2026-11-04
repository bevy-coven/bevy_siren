//! The README's quick start is `examples/quick_start.rs`, the file `tests/quick_start.rs` **runs**, so the copy a reader pastes is the copy that plays.
//! The crate-root doc's block is held to the same file in every build (`quick_start::the_crate_root_doc_is_the_example`); this is the README's half, behind a feature
//! because it reads `README.md`. Run with `cargo test --features readme-example --test all readme::`.
#![cfg(feature = "readme-example")]

const README: &str = include_str!("../README.md");
const EXAMPLE: &str = include_str!("../examples/quick_start.rs");

/// The ```` ```rust ```` block under `## Quick start` in the README. The fence's marker (`rust` or
/// `rust,no_run`) is not compared: the README's block is `no_run` because a reader must not have a window opened
/// by a doc test.
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

#[test]
fn the_readme_quick_start_is_the_example() {
    assert_eq!(
        readme_block(),
        EXAMPLE.lines().collect::<Vec<_>>(),
        "README.md's quick start differs from examples/quick_start.rs, the file the tests run"
    );
}
