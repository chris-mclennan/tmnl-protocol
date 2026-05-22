---
name: verify
description: Run the tmnl-protocol verification gate — cargo fmt, build, clippy (warning-free), and the round-trip test suite — and report. Use after making changes, before committing.
allowed-tools: Bash(cargo fmt:*), Bash(cargo build:*), Bash(cargo clippy:*), Bash(cargo test:*)
---

# Verify tmnl-protocol

Run the standard gate, in order, and stop at the first failure:

1. `cargo fmt` — format (this rewrites files; that's expected).
2. `cargo build` — must compile clean.
3. `cargo clippy --all-targets` — must be **warning-free**.
4. `cargo test` — all tests pass.

Report the outcome of each step. If a build/test fails, surface the error —
don't paper over it.

This crate is a path dependency of `tmnl`, `mnml`, and `mixr`. A change to the
wire format ripples to all of them — the round-trip test for every `Message`
variant is the contract that keeps them in agreement. If you add or change a
message, there must be a round-trip test for it.
