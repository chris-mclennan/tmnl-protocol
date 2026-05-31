<div align="center">

# tmnl-protocol

**The binary wire protocol between the [tmnl](https://github.com/chris-mclennan/tmnl)
terminal and a backing app.**

A small, dependency-free, length-prefixed message format. tmnl renders the cells;
an app sends them. This crate defines that contract — exactly once — so both ends
agree.

[![Crates.io](https://img.shields.io/crates/v/tmnl-protocol.svg?logo=rust)](https://crates.io/crates/tmnl-protocol)
[![Documentation](https://docs.rs/tmnl-protocol/badge.svg)](https://docs.rs/tmnl-protocol)
[![CI](https://github.com/chris-mclennan/tmnl-protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/chris-mclennan/tmnl-protocol/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

---

In tmnl's **native mode**, an app doesn't emit ANSI escape codes — it sends
*structured cells* over a Unix socket. `tmnl-protocol` is the wire format for
that channel: a handful of message types, a binary cell layout, and a diff-run
frame encoding. It is the single source of truth shared by the `tmnl` renderer
and every backing app (such as [`mnml`](https://github.com/chris-mclennan/mnml)).

Zero dependencies. Pure `std`. Just types and two functions.

## Wire format

Every message is length-prefixed:

```
┌────────────────────┬──────────┬───────────────────────────────┐
│  payload length    │  type    │  payload                      │
│  u32, little-endian│  u8      │  type-specific, little-endian  │
│  4 bytes           │  1 byte  │  length − 1 bytes              │
└────────────────────┴──────────┴───────────────────────────────┘
```

[`read_message`] reads one whole message from any `Read`; [`write_message`]
writes one to any `Write`. Decoding is defensive — every length and count is
sanity-capped, an unknown message type is a clean `io::Error`, never a panic.

## Messages

The protocol is **version 3** (`PROTOCOL_VERSION`). Eight message types:

| Message | Direction | Purpose |
|---------|-----------|---------|
| `Hello { version }` | both | handshake — agree on the protocol version |
| `Frame(Frame)` | app → terminal | a frame of cells (full or diff-run) |
| `Resize(Resize)` | terminal → app | the grid changed size |
| `Input(InputEvent)` | terminal → app | a key or mouse event |
| `Quit` | both | tear the connection down |
| `Title(String)` | app → terminal | set this connection's tab title |
| `OpenPane { command, args }` | app → terminal | ask the terminal to spawn a sibling native pane |
| `Palette { bg, fg, accent }` | terminal → app | hand the app the host theme so it can re-theme |

A `Frame` carries a sequence number, grid dimensions, cursor state, and a list of
`DiffRun`s — each a `start` offset plus a run of `WireCell`s (`ch` / `fg` / `bg` /
`attrs`). Sending only the changed runs keeps steady-state redraws cheap.

## Usage

Add it to a backing app:

```bash
cargo add tmnl-protocol
```

A minimal native-mode client — connect, handshake, then react to the terminal:

```rust
use std::os::unix::net::UnixStream;
use tmnl_protocol::{Message, PROTOCOL_VERSION, read_message, write_message};

fn run(socket: &str) -> std::io::Result<()> {
    let mut sock = UnixStream::connect(socket)?;

    // Handshake.
    write_message(&mut sock, &Message::Hello { version: PROTOCOL_VERSION })?;
    write_message(&mut sock, &Message::Title("my app".into()))?;

    // React to what the terminal sends.
    loop {
        match read_message(&mut sock)? {
            Message::Resize(r)  => { /* re-layout to r.cols × r.rows */ }
            Message::Input(ev)  => { /* handle the key / mouse event */ }
            Message::Palette(_) => { /* optional: re-theme to the host */ }
            Message::Quit       => break,
            _ => {}
        }
        // ...and send Message::Frame(..) back to draw.
    }
    Ok(())
}
```

Colours are packed RGBA `u32`s — [`pack_rgba`], [`pack_rgba_u8`], and
[`unpack_rgba`] convert to and from the wire form.

[`read_message`]: https://docs.rs/tmnl-protocol/latest/tmnl_protocol/fn.read_message.html
[`write_message`]: https://docs.rs/tmnl-protocol/latest/tmnl_protocol/fn.write_message.html
[`pack_rgba`]: https://docs.rs/tmnl-protocol/latest/tmnl_protocol/fn.pack_rgba.html
[`pack_rgba_u8`]: https://docs.rs/tmnl-protocol/latest/tmnl_protocol/fn.pack_rgba_u8.html
[`unpack_rgba`]: https://docs.rs/tmnl-protocol/latest/tmnl_protocol/fn.unpack_rgba.html

## Stability

The protocol is pre-1.0 and still evolving. New message types are added at the
end so older peers can ignore them; the `Hello` version is how both ends detect
a mismatch. Until 1.0, treat any version bump as potentially breaking.

## The tmnl family

tmnl-protocol is one of a small family of terminal-native Rust tools:

| Project | What it is | |
|---------|-----------|--|
| [**tmnl**](https://github.com/chris-mclennan/tmnl) | A GPU-accelerated terminal | speaks this protocol as the server |
| [**mnml**](https://github.com/chris-mclennan/mnml) | A terminal IDE | speaks it as a backing app |
| [**mixr**](https://github.com/chris-mclennan/mixr) | A terminal DJ app | speaks it as a backing app |
| **tmnl-protocol** | The binary wire protocol | ← you are here |
| [**fim-engine**](https://github.com/chris-mclennan/fim-engine) | Embedded code completion | local FIM, used by tmnl & mnml |

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Because both
the terminal and every backing app depend on this crate, changes here ripple
widely; the round-trip test suite (`cargo test`) is the safety net. The roadmap
lives in [`.local/PLAN.md`](.local/PLAN.md) and the release history in
[CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
