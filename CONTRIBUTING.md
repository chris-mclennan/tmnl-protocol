# Contributing to tmnl-protocol

Thanks for your interest in tmnl-protocol. It's a small crate, but a load-bearing
one — read this before changing the wire format.

## Getting started

```bash
git clone https://github.com/chris-mclennan/tmnl-protocol
cd tmnl-protocol
cargo test
```

The crate is pure `std` — zero dependencies — and builds on stable Rust
(MSRV **1.85**, edition 2024).

## The verification gate

Every change must pass, in order:

```bash
cargo fmt
cargo build
cargo clippy --all-targets   # warning-free
cargo test
```

## Changing the wire format

Both the `tmnl` terminal **and** every backing app (`mnml`, `mixr`, …) depend on
this crate. A change to the encoding ripples to all of them at once. So:

- **Add, don't reshape.** New message types go at the *end* of the `Message` enum
  and get a new type tag. An older peer that doesn't know the tag will reject it
  cleanly — that's by design — but it won't crash.
- **Bump `PROTOCOL_VERSION`** when the handshake's meaning changes, and treat any
  bump as potentially breaking until 1.0.
- **Keep decoding defensive.** Every length and count read off the wire must be
  sanity-capped before it's used to allocate or index. A malformed payload must
  produce an `io::Error`, never a panic.
- **Round-trip test every variant.** The test suite encodes each `Message`, decodes
  it straight back, and asserts equality. Any new variant or field needs a
  round-trip test — that test *is* the contract.

## Conventions

- Run `cargo fmt` and keep `cargo clippy --all-targets` warning-free.
- Match the surrounding code style.
- Keep commits small and focused.

## Pull requests

1. Branch from `main`.
2. Make your change with round-trip tests; run the verification gate.
3. Open a PR describing the change and which peers it affects.
4. CI runs `fmt` + `clippy -D warnings` + `test` — keep it green.

## License

By contributing, you agree that your contributions will be dual licensed under
the MIT and Apache-2.0 licenses, as described in [README.md](README.md#license),
without any additional terms or conditions.
