# Changelog

All notable changes to **tmnl-protocol** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The roadmap lives in [`.local/PLAN.md`](.local/PLAN.md).

## [Unreleased]

tmnl-protocol has not yet had a tagged release. The `0.0.1` line below
summarises the current `main`.

## [0.0.1]

### Added

- **Wire format** — length-prefixed messages: a `u32` little-endian payload
  length, a `u8` type tag, then a type-specific little-endian payload.
- **`read_message` / `write_message`** — encode and decode one message over any
  `Read` / `Write`. Decoding is defensive: every length and count is
  sanity-capped, and an unknown message type is a clean error rather than a panic.
- **Protocol version 3** — eight `Message` types: `Hello`, `Frame`, `Resize`,
  `Input`, `Quit`, `Title`, `OpenPane`, `Palette`.
- **Cell & frame types** — `WireCell` (char + packed RGBA fg/bg + attrs),
  `DiffRun` (a contiguous run of changed cells), and `Frame` (sequence number,
  dimensions, cursor state, and a list of runs).
- **Input types** — `KeyCode`, `KeyInput`, `MouseKind`, `MouseInput`, and the
  `InputEvent` union, plus modifier / key / mouse / button constants.
- **RGBA helpers** — `pack_rgba`, `pack_rgba_u8`, and `unpack_rgba`.
- A round-trip test for every `Message` variant — the contract both ends rely on.

[Unreleased]: https://github.com/chris-mclennan/tmnl-protocol/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/chris-mclennan/tmnl-protocol/releases/tag/v0.0.1
