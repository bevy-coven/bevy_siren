#!/bin/bash
# The full matrix from AGENTS.md, one line per command, cleaning this crate's own artifacts when the disk is short.
# Run it as:  CARGO_INCREMENTAL=0 bash docs/loop/matrix.sh      (about four and a half minutes)
#
# It started as iteration 112's script (docs/loop/runs/2026-11-03-112/matrix.sh) and has one change, made in iteration 122:
# `cargo test --all-features` is two commands here. That one command links the lib's tests, `all` AND every example in
# parallel, and it is the step that failed to link at 3.9 GB free in iteration 121 (re-run by hand it took the disk to
# 125 MB). `cargo test --all-features --tests -j 1` runs the same tests one link at a time and passed at 3.0 GB; the
# examples are built, with the same features, by the `cargo build --examples --all-features` step further down.
cd "$(git rev-parse --show-toplevel)"
free_gb() { df -k ~ | awk 'NR==2{printf "%.1f", $4/1048576}'; }
guard() { if [ "$(df -k ~ | awk 'NR==2{print int($4/1048576)}')" -lt 5 ]; then cargo clean -p bevy_siren --target-dir target >/dev/null 2>&1; echo "   (disk was short: removed this crate's artifacts; free now $(free_gb) GB)"; fi; }
run() {
  echo "=== $*   [free $(free_gb) GB, $(date +%H:%M:%S)]"
  "$@" > /tmp/siren-matrix-step.log 2>&1; code=$?
  grep -E "^test result|^error(\[|:)|^warning: (unused|unreachable)|FAILED|panicked at|Finished|generated [0-9]+ warning" /tmp/siren-matrix-step.log | sort | uniq -c | sort -rn | head -8
  echo "   exit: $code"
  guard
}
cargo clean -p bevy_siren --target-dir target >/dev/null 2>&1; echo "(started clean of this crate's artifacts; free $(free_gb) GB)"
run cargo build
run cargo test
run cargo test --features serde
run cargo test --features semantic
run cargo test --features mixer
run cargo test --all-features --tests -j 1
run cargo test --doc
run cargo test --features readme-example --test all readme::
run cargo build --examples
run cargo build --examples --all-features
run cargo clippy --all-targets --all-features -- -D warnings
run cargo fmt --check
RUSTDOCFLAGS="-D warnings" run cargo doc --no-deps --all-features
echo "=== matrix done $(date +%H:%M:%S)"
