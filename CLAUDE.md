# tmnl-protocol — working notes

The binary wire protocol between the `tmnl` terminal and a backing app (`mnml`,
`mixr`). One small crate, but a load-bearing one: it's a path dependency of
three other repos, so a change here ripples to all of them at once.

## Architecture

A single file, `src/lib.rs`. The whole crate is:

- The `Message` enum (8 variants) plus the cell / frame / input types.
- `write_message<W: Write>` / `read_message<R: Read>` — the codec.
- Length-prefixed framing: `[u32 LE payload length][u8 type][payload]`.
- RGBA pack/unpack helpers.

Zero dependencies — pure `std`. Keep it that way.

## Invariants — do not break these

1. **Additive only.** New message types append to the `Message` enum with a new
   type tag. Never reshape an existing message — that's a silent breakage across
   every peer. An older peer rejecting an unknown tag is *correct* behaviour.
2. **Defensive decoding.** Every length and count read off the wire is
   sanity-capped before it's used to allocate or index. Malformed input must
   produce an `io::Error`, never a panic.
3. **Round-trip tests are the spec.** Every `Message` variant has a test that
   encodes it, decodes it straight back, and asserts equality. A new variant or
   field is not done until it has one.
4. **Bump `PROTOCOL_VERSION`** when the handshake's meaning changes.

## Verify

`cargo fmt` · `cargo build` · `cargo clippy --all-targets` (warning-free) ·
`cargo test`. The `/verify` skill in `.claude/skills/` runs the gate.

## Conventions

- `cargo fmt` + `cargo clippy --all-targets` clean before every commit.
- Commit messages end with the `Co-Authored-By: Claude …` trailer.
- Roadmap lives in `.local/PLAN.md`; user-facing history in `CHANGELOG.md`.
