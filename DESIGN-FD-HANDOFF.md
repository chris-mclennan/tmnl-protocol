# Pty-fd handoff (SCM_RIGHTS) — design

Adds the wire-level coordination for transferring a *running* pty
session from one process to another via Unix-socket `SCM_RIGHTS`
file-descriptor passing. Use case: mnml is running `claude` (or
`codex`, or a shell) in a `Pane::Pty`; the user wants to pop that
session out into a dedicated tmnl tab without losing scrollback or
restarting the child process.

## Constraint

`SCM_RIGHTS` is a Unix-socket-only mechanism for passing open file
descriptors between processes. The descriptors aren't part of the
byte stream — they ride along as ancillary data on a `sendmsg(2)`
call, and the receiver pulls them out of the cmsg buffer on
`recvmsg(2)`. This means:

- A regular `write_message(&mut Write, msg)` cannot attach an fd.
  `Write` is just a byte sink; there's no way to thread ancillary
  data through it.
- The fd-passing API has to operate on a `UnixStream` (or anything
  with an `AsRawFd`) and call `sendmsg`/`recvmsg` directly.
- `BufReader<UnixStream>` cannot be used on streams that carry fd
  ancillary data — the buffer would consume past the ancillary
  boundary and the cmsg would never be visible to the recvmsg call.

So the protocol grows a parallel pair of helpers:

```rust
pub fn send_message_with_fd<S: AsRawFd>(
    stream: &S,
    msg: &Message,
    fd: Option<RawFd>,
) -> io::Result<()>;

pub fn read_message_with_fd<S: AsRawFd>(
    stream: &S,
) -> io::Result<(Message, Option<RawFd>)>;
```

Connections that want fd-passing use these instead of `write_message`
/ `read_message`. Mixing the two on the same stream is undefined (the
BufReader-buffering hazard above). In practice we expect fd-passing
connections to be one-shot — the sender sends one message-with-fd and
closes the connection.

## New `Message` variant

```rust
Message::OpenPaneTransfer {
    command: String,   // for the new tab's title chip + debug label
    args: Vec<String>, // ditto
}
```

The command + args are descriptive only — tmnl doesn't spawn anything
new. The attached fd IS the pty master; tmnl assumes ownership and
hooks it into its existing pty render pipeline as if it had spawned
the child itself.

Serialization: same shape as `Message::OpenPane`, just a different
`MSG_*` tag. Code:

```
[len:u32 LE][tag:u8 = MSG_OPEN_PANE_TRANSFER][cmd_len:u32][cmd:utf8]
[n_args:u32][...arg_len:u32, arg:utf8]
```

`PROTOCOL_VERSION` bumps to `4`. Receivers on `v3` see the new tag,
fail to decode, drop the message. Senders should not send
`OpenPaneTransfer` to a `v3` receiver — version check on Hello.

## Lifecycle of a handoff

1. **mnml**: user invokes `:tmnl.pop-pty` (or clicks a "pop out"
   chip) on the focused `Pane::Pty`.
2. **mnml**: extracts the pty master fd from the `PtySession`. The
   child process is unaware of any of this — it just talks to its
   pty as normal.
3. **mnml**: opens a connection to tmnl's native-client UDS (the
   same one tmnl listens on for `--mnml`/`--blit` clients).
4. **mnml**: `send_message_with_fd(&stream, &Message::OpenPaneTransfer
   { command, args }, Some(pty_fd))`.
5. **mnml**: closes the connection. Stops reading from the pty.
   `dup2`'s the pty fd out of its file descriptor table (or just
   closes its copy — the fd was passed by `dup`-ing into the cmsg).
   Removes the `Pane::Pty`. The child process is now talking to a
   pty whose other end is owned by tmnl.
6. **tmnl**: accepts the connection on its UDS.
7. **tmnl**: `read_message_with_fd(&stream)` →
   `(OpenPaneTransfer { ... }, Some(received_fd))`.
8. **tmnl**: opens a new native pane, wires `received_fd` as its
   pty master. The child's output stream is now driving tmnl's
   render, not mnml's vt100 parser.

The child process never knows the handoff happened. Its `read()` /
`write()` calls on the pty continue to work — they were always
talking to the kernel's pty layer, not to either parent process
directly.

## What's *not* in the v0.0.2 protocol commit

The new Message variant + the helpers ship in tmnl-protocol v0.0.2.
The consumer-side work — mnml's `:tmnl.pop-pty` and tmnl's accept-
side handler — lands in their own commits (mnml-rs and tmnl-rs).

The fd-handoff use case is the motivating one, but the helpers are
generic: any tmnl-protocol exchange that needs to attach an fd to a
message can use `send_message_with_fd`. Future uses (passing a
listening socket between processes, etc.) compose without protocol
changes.

## Implementation notes

- Use `libc::sendmsg` / `libc::recvmsg` directly. We add a
  `[target.'cfg(unix)'.dependencies] libc = "0.2"` dep — already
  ubiquitous in the Rust ecosystem.
- The cmsg buffer is sized via `libc::CMSG_SPACE(sizeof(c_int))`
  once at function start.
- One fd per message — extending to multiple fds is YAGNI for now.
- Errors map to `io::Error` with the underlying errno preserved.
- `Some(RawFd)` returned by `read_message_with_fd` transfers
  ownership to the caller. The caller is responsible for closing
  it (or wrapping it in a `OwnedFd` / `UnixStream::from_raw_fd`).
- Non-Unix platforms get an explicit "not supported" error at the
  helper level — the API exists everywhere so downstream code
  doesn't `#[cfg(unix)]`-fork, but a Windows caller gets a clean
  io::ErrorKind::Unsupported.
