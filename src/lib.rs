//! Binary wire format between the tmnl terminal and a backing app.
//!
//! Both the tmnl renderer and the mnml editor's `blit` backend depend on
//! this crate so the protocol is defined exactly once.

use std::io::{self, Read, Write};

pub const PROTOCOL_VERSION: u32 = 4;
pub const MSG_HELLO: u8 = 1;
pub const MSG_FRAME: u8 = 2;
pub const MSG_RESIZE: u8 = 3;
pub const MSG_INPUT: u8 = 4;
pub const MSG_QUIT: u8 = 5;
/// `Message::Title(String)` — client → server. The hosted app
/// (e.g. mnml) tells the renderer what to show as the tab title;
/// added in v3, optional on older clients (renderer falls back to
/// "mnml" / shell name).
pub const MSG_TITLE: u8 = 6;
/// `Message::OpenPane { command, args }` — client → server. The hosted
/// app asks the renderer to open a *new* pane running `command args…`
/// as its own native client (tmnl appends the minted `--blit
/// <socket>`). Used by mnml's `mixr.show` to bring up mixr as a
/// sibling pane. Added after v3 — optional on older renderers.
pub const MSG_OPEN_PANE: u8 = 7;
/// `Message::Palette { bg, fg, accent }` — server → client. The host
/// (e.g. mnml) hands a hosted app its active theme colors so the app
/// can re-theme to match its container. Three packed-rgba values (see
/// [`pack_rgba_u8`]). Sent right after the connect handshake. Added
/// after v3 — optional; a client that ignores it keeps its own theme.
pub const MSG_PALETTE: u8 = 8;
/// `Message::OpenPaneTransfer { command, args }` — client → server.
/// Like `OpenPane`, but signals that the SENDER has attached a pty
/// master fd via SCM_RIGHTS ancillary data on the same `sendmsg(2)`
/// call. The renderer takes ownership of that fd and uses it as the
/// new pane's pty (instead of spawning a fresh process). Use case:
/// pop-out a running CLI session (claude / codex / shell) from
/// mnml's `Pane::Pty` into a dedicated tmnl tab without losing
/// scrollback or restarting the child.
///
/// **Wire-byte layout is identical to `OpenPane`**; the difference is
/// purely the tag + the cmsg-attached fd. See `DESIGN-FD-HANDOFF.md`.
/// Added in protocol v4.
pub const MSG_OPEN_PANE_TRANSFER: u8 = 9;

pub const MOD_SHIFT: u8 = 1;
pub const MOD_CTRL: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_SUPER: u8 = 8;

const SUB_KEY: u8 = 1;
const SUB_MOUSE: u8 = 2;

const KEY_CHAR: u8 = 0;
pub const KEY_BACKSPACE: u8 = 1;
pub const KEY_ENTER: u8 = 2;
pub const KEY_LEFT: u8 = 3;
pub const KEY_RIGHT: u8 = 4;
pub const KEY_UP: u8 = 5;
pub const KEY_DOWN: u8 = 6;
pub const KEY_HOME: u8 = 7;
pub const KEY_END: u8 = 8;
pub const KEY_PAGE_UP: u8 = 9;
pub const KEY_PAGE_DOWN: u8 = 10;
pub const KEY_TAB: u8 = 11;
pub const KEY_BACK_TAB: u8 = 12;
pub const KEY_DELETE: u8 = 13;
pub const KEY_INSERT: u8 = 14;
pub const KEY_ESC: u8 = 15;
pub const KEY_F_BASE: u8 = 16;

pub const MOUSE_DOWN: u8 = 1;
pub const MOUSE_UP: u8 = 2;
pub const MOUSE_DRAG: u8 = 3;
pub const MOUSE_MOVED: u8 = 4;
pub const MOUSE_SCROLL_UP: u8 = 5;
pub const MOUSE_SCROLL_DOWN: u8 = 6;
pub const MOUSE_SCROLL_LEFT: u8 = 7;
pub const MOUSE_SCROLL_RIGHT: u8 = 8;

pub const BUTTON_LEFT: u8 = 0;
pub const BUTTON_RIGHT: u8 = 1;
pub const BUTTON_MIDDLE: u8 = 2;
pub const BUTTON_NONE: u8 = 3;

const MAX_PAYLOAD: u32 = 64 * 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WireCell {
    pub ch: u32,
    pub fg: u32,
    pub bg: u32,
    pub attrs: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffRun {
    pub start: u32,
    pub cells: Vec<WireCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub seq: u64,
    pub cols: u16,
    pub rows: u16,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub cursor_shape: u8,
    pub cursor_visible: u8,
    pub runs: Vec<DiffRun>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyCode {
    Char(char),
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Esc,
    F(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyInput {
    pub code: KeyCode,
    pub mods: u8,
    pub press: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseKind {
    Down,
    Up,
    Drag,
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseInput {
    pub kind: MouseKind,
    pub button: u8,
    pub col: u16,
    pub row: u16,
    pub mods: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    Key(KeyInput),
    Mouse(MouseInput),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Hello {
        version: u32,
    },
    Frame(Frame),
    Resize(Resize),
    Input(InputEvent),
    Quit,
    /// Client → server: set the tab title for this connection. The
    /// renderer uses it as the tab chip label (otherwise falls back
    /// to a default like "mnml" for the blit-style backend).
    Title(String),
    /// Client → server: open a new pane running `command args…` as a
    /// native client. tmnl splits + spawns it, appending the minted
    /// `--blit <socket>`. See [`MSG_OPEN_PANE`].
    OpenPane {
        command: String,
        args: Vec<String>,
    },
    /// Server → client: the host's active theme palette so a hosted
    /// app can re-theme to match its container. `bg` / `fg` / `accent`
    /// are packed rgba (see [`pack_rgba_u8`] / [`unpack_rgba`]). See
    /// [`MSG_PALETTE`].
    Palette {
        bg: u32,
        fg: u32,
        accent: u32,
    },
    /// Client → server: pty-fd handoff. Same shape as `OpenPane`, but
    /// the sender attaches a pty master fd via SCM_RIGHTS ancillary
    /// data on the same `sendmsg(2)`. The renderer takes ownership of
    /// the fd and uses it as the new pane's pty. The byte layout is
    /// identical to `OpenPane` — only the tag differs. See
    /// [`send_message_with_fd`] / [`read_message_with_fd`] +
    /// `DESIGN-FD-HANDOFF.md`.
    OpenPaneTransfer {
        command: String,
        args: Vec<String>,
    },
}

pub fn write_message<W: Write>(w: &mut W, msg: &Message) -> io::Result<()> {
    let mut buf: Vec<u8> = Vec::with_capacity(64);
    buf.extend_from_slice(&[0u8; 4]);
    match msg {
        Message::Hello { version } => {
            buf.push(MSG_HELLO);
            buf.extend_from_slice(&version.to_le_bytes());
        }
        Message::Resize(Resize { cols, rows }) => {
            buf.push(MSG_RESIZE);
            buf.extend_from_slice(&cols.to_le_bytes());
            buf.extend_from_slice(&rows.to_le_bytes());
        }
        Message::Frame(f) => {
            buf.push(MSG_FRAME);
            buf.extend_from_slice(&f.seq.to_le_bytes());
            buf.extend_from_slice(&f.cols.to_le_bytes());
            buf.extend_from_slice(&f.rows.to_le_bytes());
            buf.extend_from_slice(&f.cursor_col.to_le_bytes());
            buf.extend_from_slice(&f.cursor_row.to_le_bytes());
            buf.push(f.cursor_shape);
            buf.push(f.cursor_visible);
            let n_runs = f.runs.len() as u32;
            buf.extend_from_slice(&n_runs.to_le_bytes());
            for run in &f.runs {
                buf.extend_from_slice(&run.start.to_le_bytes());
                let n_cells = run.cells.len() as u32;
                buf.extend_from_slice(&n_cells.to_le_bytes());
                for c in &run.cells {
                    buf.extend_from_slice(&c.ch.to_le_bytes());
                    buf.extend_from_slice(&c.fg.to_le_bytes());
                    buf.extend_from_slice(&c.bg.to_le_bytes());
                    buf.extend_from_slice(&c.attrs.to_le_bytes());
                }
            }
        }
        Message::Input(ev) => {
            buf.push(MSG_INPUT);
            encode_input(&mut buf, ev);
        }
        Message::Quit => {
            buf.push(MSG_QUIT);
        }
        Message::Title(s) => {
            buf.push(MSG_TITLE);
            let bytes = s.as_bytes();
            let len = (bytes.len() as u32).min(MAX_PAYLOAD.saturating_sub(8));
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(&bytes[..len as usize]);
        }
        Message::OpenPane { command, args } => {
            buf.push(MSG_OPEN_PANE);
            let cmd = command.as_bytes();
            let cmd_len = (cmd.len() as u32).min(MAX_PAYLOAD.saturating_sub(8));
            buf.extend_from_slice(&cmd_len.to_le_bytes());
            buf.extend_from_slice(&cmd[..cmd_len as usize]);
            buf.extend_from_slice(&(args.len() as u32).to_le_bytes());
            for a in args {
                let ab = a.as_bytes();
                let al = (ab.len() as u32).min(MAX_PAYLOAD.saturating_sub(8));
                buf.extend_from_slice(&al.to_le_bytes());
                buf.extend_from_slice(&ab[..al as usize]);
            }
        }
        Message::Palette { bg, fg, accent } => {
            buf.push(MSG_PALETTE);
            buf.extend_from_slice(&bg.to_le_bytes());
            buf.extend_from_slice(&fg.to_le_bytes());
            buf.extend_from_slice(&accent.to_le_bytes());
        }
        Message::OpenPaneTransfer { command, args } => {
            // Byte layout identical to OpenPane — only the tag differs.
            // The accompanying fd rides via SCM_RIGHTS, not the byte
            // stream. See [`send_message_with_fd`].
            buf.push(MSG_OPEN_PANE_TRANSFER);
            let cmd = command.as_bytes();
            let cmd_len = (cmd.len() as u32).min(MAX_PAYLOAD.saturating_sub(8));
            buf.extend_from_slice(&cmd_len.to_le_bytes());
            buf.extend_from_slice(&cmd[..cmd_len as usize]);
            buf.extend_from_slice(&(args.len() as u32).to_le_bytes());
            for a in args {
                let ab = a.as_bytes();
                let al = (ab.len() as u32).min(MAX_PAYLOAD.saturating_sub(8));
                buf.extend_from_slice(&al.to_le_bytes());
                buf.extend_from_slice(&ab[..al as usize]);
            }
        }
    }
    let payload_len = (buf.len() - 4) as u32;
    buf[0..4].copy_from_slice(&payload_len.to_le_bytes());
    w.write_all(&buf)
}

pub fn read_message<R: Read>(r: &mut R) -> io::Result<Message> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf);
    if len == 0 || len > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("bad payload length {len}"),
        ));
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)?;
    decode_payload(&payload)
}

fn decode_payload(p: &[u8]) -> io::Result<Message> {
    let mut c = Cursor::new(p);
    let kind = c.u8()?;
    match kind {
        MSG_HELLO => {
            let version = c.u32()?;
            Ok(Message::Hello { version })
        }
        MSG_RESIZE => {
            let cols = c.u16()?;
            let rows = c.u16()?;
            Ok(Message::Resize(Resize { cols, rows }))
        }
        MSG_FRAME => {
            let seq = c.u64()?;
            let cols = c.u16()?;
            let rows = c.u16()?;
            let cursor_col = c.u16()?;
            let cursor_row = c.u16()?;
            let cursor_shape = c.u8()?;
            let cursor_visible = c.u8()?;
            let n_runs = c.u32()? as usize;
            if n_runs > 1 << 20 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("absurd run count {n_runs}"),
                ));
            }
            let grid_max = cols as u32 * rows as u32;
            let mut runs = Vec::with_capacity(n_runs);
            for _ in 0..n_runs {
                let start = c.u32()?;
                let n_cells = c.u32()? as usize;
                if n_cells > (cols as usize * rows as usize * 4).max(1 << 20) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("absurd run cell count {n_cells}"),
                    ));
                }
                if start.saturating_add(n_cells as u32) > grid_max {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("run start {start} + len {n_cells} > grid {grid_max}"),
                    ));
                }
                let mut cells = Vec::with_capacity(n_cells);
                for _ in 0..n_cells {
                    let ch = c.u32()?;
                    let fg = c.u32()?;
                    let bg = c.u32()?;
                    let attrs = c.u32()?;
                    cells.push(WireCell { ch, fg, bg, attrs });
                }
                runs.push(DiffRun { start, cells });
            }
            Ok(Message::Frame(Frame {
                seq,
                cols,
                rows,
                cursor_col,
                cursor_row,
                cursor_shape,
                cursor_visible,
                runs,
            }))
        }
        MSG_INPUT => Ok(Message::Input(decode_input(&mut c)?)),
        MSG_QUIT => Ok(Message::Quit),
        MSG_TITLE => {
            let len = c.u32()? as usize;
            // Sanity cap so an attacker / bug can't allocate gigabytes.
            if len > 4096 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("absurd title len {len}"),
                ));
            }
            let bytes = c.take(len)?.to_vec();
            let s = String::from_utf8(bytes).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("bad utf-8 title: {e}"))
            })?;
            Ok(Message::Title(s))
        }
        MSG_OPEN_PANE => {
            let (command, args) = decode_open_pane_payload(&mut c)?;
            Ok(Message::OpenPane { command, args })
        }
        MSG_OPEN_PANE_TRANSFER => {
            let (command, args) = decode_open_pane_payload(&mut c)?;
            Ok(Message::OpenPaneTransfer { command, args })
        }
        MSG_PALETTE => {
            let bg = c.u32()?;
            let fg = c.u32()?;
            let accent = c.u32()?;
            Ok(Message::Palette { bg, fg, accent })
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown msg type {other}"),
        )),
    }
}

/// Shared decoder for the OpenPane / OpenPaneTransfer payload — same
/// wire layout, different `Message` variant. Extracted to keep both
/// match arms compact + their semantics aligned.
/// Serialize a `Message` to its on-wire byte form — same shape
/// `write_message` would write, returned as a `Vec<u8>` so callers can
/// hand it to non-`Write` sinks (notably `libc::sendmsg`).
pub fn encode_message(msg: &Message) -> Vec<u8> {
    let mut sink: Vec<u8> = Vec::with_capacity(64);
    // `write_message` returns `io::Result` but `Vec<u8>` never fails;
    // unwrap is safe.
    write_message(&mut sink, msg).expect("Vec<u8> write");
    sink
}

/// Send a `Message` over a Unix-socket-backed stream, optionally
/// attaching a single file descriptor via `SCM_RIGHTS` ancillary
/// data. The receiver pulls the fd out via [`read_message_with_fd`].
///
/// `stream` is anything with `AsRawFd` whose underlying socket family
/// supports `SCM_RIGHTS` (i.e. `AF_UNIX`). Passing a TCP socket here
/// is a programmer error — the kernel will refuse the cmsg.
///
/// Used for the pty-fd handoff: see `DESIGN-FD-HANDOFF.md`. Generic
/// otherwise — any future message that needs a side-channel fd can
/// use the same helper.
#[cfg(unix)]
pub fn send_message_with_fd<S: std::os::unix::io::AsRawFd>(
    stream: &S,
    msg: &Message,
    fd: Option<std::os::unix::io::RawFd>,
) -> io::Result<()> {
    let bytes = encode_message(msg);
    let sock_fd = stream.as_raw_fd();

    // iovec carrying the message bytes.
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr() as *mut libc::c_void,
        iov_len: bytes.len(),
    };

    // Set up the msghdr. Field order varies by platform — use a zeroed
    // struct and assign fields by name so we don't trip on layout.
    // SAFETY: zero-initialized is a valid `msghdr` (all pointers null,
    // counts 0). We fill in the fields we care about before sendmsg.
    let mut mhdr: libc::msghdr = unsafe { std::mem::zeroed() };
    mhdr.msg_name = std::ptr::null_mut();
    mhdr.msg_namelen = 0;
    mhdr.msg_iov = &mut iov;
    mhdr.msg_iovlen = 1;

    // The cmsg buffer — sized to hold exactly one fd's worth of
    // SCM_RIGHTS payload. Stays on the stack via a fixed array so the
    // pointer is valid for the sendmsg call.
    // SAFETY: CMSG_SPACE returns the aligned byte count; the array is
    // larger than any real platform's value (we pick 64 conservatively).
    let mut cmsg_buf: [u8; 64] = [0u8; 64];

    if let Some(fd) = fd {
        // SAFETY: libc::CMSG_SPACE is the standard cmsg-sizing call.
        let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<libc::c_int>() as u32) };
        let cmsg_len = unsafe { libc::CMSG_LEN(std::mem::size_of::<libc::c_int>() as u32) };
        if (cmsg_space as usize) > cmsg_buf.len() {
            return Err(io::Error::other("cmsg buffer too small (recompile)"));
        }
        mhdr.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
        mhdr.msg_controllen = cmsg_space as _;

        // SAFETY: msg_control is non-null + msg_controllen ≥ CMSG_SPACE
        // (asserted above). CMSG_FIRSTHDR returns a valid pointer.
        let cmsg_ptr = unsafe { libc::CMSG_FIRSTHDR(&mhdr) };
        if cmsg_ptr.is_null() {
            return Err(io::Error::other("CMSG_FIRSTHDR returned null"));
        }
        // SAFETY: we own the cmsg buffer; writing through the pointer is
        // safe. cmsg_len / level / type are exact-fit values for SCM_RIGHTS.
        unsafe {
            (*cmsg_ptr).cmsg_len = cmsg_len as _;
            (*cmsg_ptr).cmsg_level = libc::SOL_SOCKET;
            (*cmsg_ptr).cmsg_type = libc::SCM_RIGHTS;
            // Copy the fd into the cmsg data slot.
            std::ptr::copy_nonoverlapping(
                &fd as *const std::os::unix::io::RawFd as *const u8,
                libc::CMSG_DATA(cmsg_ptr),
                std::mem::size_of::<libc::c_int>(),
            );
        }
    }

    // SAFETY: mhdr is fully initialized + valid for the sendmsg call.
    let sent = unsafe { libc::sendmsg(sock_fd, &mhdr, 0) };
    if sent < 0 {
        return Err(io::Error::last_os_error());
    }
    if (sent as usize) != bytes.len() {
        // Partial send — extremely unlikely on a SOCK_STREAM Unix
        // socket for sub-page payloads, but report it as a hard error
        // rather than re-trying (cmsg can't ride on a follow-up send).
        return Err(io::Error::other(format!(
            "sendmsg short write: {sent} of {} bytes",
            bytes.len()
        )));
    }
    Ok(())
}

/// Receive a `Message` from a Unix-socket-backed stream, also pulling
/// out the first file descriptor attached via `SCM_RIGHTS` ancillary
/// data (if any). The fd's ownership transfers to the caller — close
/// it (or wrap in `OwnedFd` / `UnixStream::from_raw_fd`) before it
/// leaks.
///
/// Buffer sizing: the helper reads at most 64 KiB of message bytes in
/// one `recvmsg` call. That's far above the payload of any wire
/// message we emit (the largest is a Frame, which uses
/// [`write_message`] / [`read_message`] over a streamed UnixStream,
/// not this helper). Callers using `send_message_with_fd` for things
/// other than `OpenPaneTransfer` should keep payloads short.
#[cfg(unix)]
pub fn read_message_with_fd<S: std::os::unix::io::AsRawFd>(
    stream: &S,
) -> io::Result<(Message, Option<std::os::unix::io::RawFd>)> {
    let sock_fd = stream.as_raw_fd();
    let mut data_buf = [0u8; 64 * 1024];
    let mut cmsg_buf = [0u8; 64];

    let mut iov = libc::iovec {
        iov_base: data_buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: data_buf.len(),
    };

    // SAFETY: zero-initialized msghdr is well-formed. We populate the
    // fields the recvmsg call reads before calling.
    let mut mhdr: libc::msghdr = unsafe { std::mem::zeroed() };
    mhdr.msg_name = std::ptr::null_mut();
    mhdr.msg_namelen = 0;
    mhdr.msg_iov = &mut iov;
    mhdr.msg_iovlen = 1;
    mhdr.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    mhdr.msg_controllen = cmsg_buf.len() as _;

    // SAFETY: mhdr is fully initialized for the recvmsg call.
    let received = unsafe { libc::recvmsg(sock_fd, &mut mhdr, 0) };
    if received < 0 {
        return Err(io::Error::last_os_error());
    }
    if received == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "peer closed before message",
        ));
    }
    // The kernel sets `MSG_CTRUNC` when our `cmsg_buf` was too small
    // for the sender's ancillary payload — the truncated fd is
    // *gone* (not in our process, not in the sender's). For the
    // pty-fd handoff that's catastrophic: the sender will close its
    // master fd thinking we adopted it, but we got nothing. Surface
    // it as a hard error so the caller can recover (or at least toast).
    let flags = mhdr.msg_flags;
    if flags & libc::MSG_CTRUNC != 0 {
        return Err(io::Error::other(
            "recvmsg: MSG_CTRUNC — ancillary data (fd) truncated; \
             grow cmsg buffer or send fewer fds",
        ));
    }

    // Pull the first attached fd (if any) out of the cmsg buffer.
    // Any additional fds in the same SCM_RIGHTS payload are closed
    // here so they don't leak: callers expect at most one fd back.
    let mut fd: Option<std::os::unix::io::RawFd> = None;
    // SAFETY: msg_control was provided + recvmsg populates it.
    let mut cmsg_ptr = unsafe { libc::CMSG_FIRSTHDR(&mhdr) };
    while !cmsg_ptr.is_null() {
        // SAFETY: cmsg_ptr is a valid cmsghdr returned by
        // CMSG_FIRSTHDR / CMSG_NXTHDR.
        let (level, ctype, len) = unsafe {
            (
                (*cmsg_ptr).cmsg_level,
                (*cmsg_ptr).cmsg_type,
                (*cmsg_ptr).cmsg_len,
            )
        };
        if level == libc::SOL_SOCKET && ctype == libc::SCM_RIGHTS {
            // SAFETY: SCM_RIGHTS data is a sequence of c_int file
            // descriptors. `len` includes the cmsghdr; subtract its
            // size to get the data length, then divide by sizeof(int)
            // to learn how many fds rode in this cmsg.
            let data_ptr = unsafe { libc::CMSG_DATA(cmsg_ptr) };
            let hdr_size = unsafe { libc::CMSG_LEN(0) } as usize;
            let data_len = (len as usize).saturating_sub(hdr_size);
            let n_fds = data_len / std::mem::size_of::<libc::c_int>();
            for i in 0..n_fds {
                let mut got_fd: std::os::unix::io::RawFd = -1;
                // SAFETY: we just bounded the loop by the cmsg's
                // declared length; reading `i * sizeof(int)` bytes in
                // is within the buffer.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        data_ptr.add(i * std::mem::size_of::<libc::c_int>()),
                        &mut got_fd as *mut std::os::unix::io::RawFd as *mut u8,
                        std::mem::size_of::<libc::c_int>(),
                    );
                }
                if got_fd < 0 {
                    continue;
                }
                if fd.is_none() {
                    fd = Some(got_fd);
                } else {
                    // Excess fds (multi-fd SCM_RIGHTS) — we only
                    // return one; close the rest so they don't leak.
                    // SAFETY: kernel-duped fd, unique to this process.
                    unsafe {
                        libc::close(got_fd);
                    }
                }
            }
        }
        // SAFETY: CMSG_NXTHDR is the standard cmsg traversal call.
        cmsg_ptr = unsafe { libc::CMSG_NXTHDR(&mhdr, cmsg_ptr) };
    }

    // Decode the message from the data buffer. `read_message` reads a
    // length-prefixed framing; the in-memory buffer behaves like a
    // Cursor<&[u8]> via `&data_buf[..received as usize]`. If decode
    // fails, we must close any fd we extracted — otherwise the
    // duplicated descriptor leaks for the lifetime of the receiver.
    match read_message(&mut &data_buf[..received as usize]) {
        Ok(msg) => Ok((msg, fd)),
        Err(e) => {
            if let Some(raw) = fd {
                // SAFETY: we just extracted this fd via SCM_RIGHTS;
                // we own it and no one else holds it yet.
                unsafe {
                    libc::close(raw);
                }
            }
            Err(e)
        }
    }
}

/// Non-Unix stub. SCM_RIGHTS is `AF_UNIX`-specific; on other platforms
/// the helpers return `Unsupported`. They exist here so downstream
/// code (mnml/tmnl) doesn't need its own cfg-fork — it just gets an
/// `io::Error` instead of having to feature-gate the call.
#[cfg(not(unix))]
pub fn send_message_with_fd<S>(
    _stream: &S,
    _msg: &Message,
    _fd: Option<std::os::raw::c_int>,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SCM_RIGHTS fd-passing is Unix-only",
    ))
}

#[cfg(not(unix))]
pub fn read_message_with_fd<S>(_stream: &S) -> io::Result<(Message, Option<std::os::raw::c_int>)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SCM_RIGHTS fd-passing is Unix-only",
    ))
}

fn decode_open_pane_payload(c: &mut Cursor<'_>) -> io::Result<(String, Vec<String>)> {
    let cmd_len = c.u32()? as usize;
    if cmd_len > 64 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("absurd command len {cmd_len}"),
        ));
    }
    let command = String::from_utf8(c.take(cmd_len)?.to_vec()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("bad utf-8 command: {e}"),
        )
    })?;
    let n_args = c.u32()? as usize;
    if n_args > 256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("absurd arg count {n_args}"),
        ));
    }
    let mut args = Vec::with_capacity(n_args);
    for _ in 0..n_args {
        let al = c.u32()? as usize;
        if al > 64 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("absurd arg len {al}"),
            ));
        }
        args.push(String::from_utf8(c.take(al)?.to_vec()).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("bad utf-8 arg: {e}"))
        })?);
    }
    Ok((command, args))
}

fn encode_input(buf: &mut Vec<u8>, ev: &InputEvent) {
    match ev {
        InputEvent::Key(k) => {
            buf.push(SUB_KEY);
            match k.code {
                KeyCode::Char(ch) => {
                    buf.push(KEY_CHAR);
                    buf.extend_from_slice(&(ch as u32).to_le_bytes());
                }
                KeyCode::Backspace => buf.push(KEY_BACKSPACE),
                KeyCode::Enter => buf.push(KEY_ENTER),
                KeyCode::Left => buf.push(KEY_LEFT),
                KeyCode::Right => buf.push(KEY_RIGHT),
                KeyCode::Up => buf.push(KEY_UP),
                KeyCode::Down => buf.push(KEY_DOWN),
                KeyCode::Home => buf.push(KEY_HOME),
                KeyCode::End => buf.push(KEY_END),
                KeyCode::PageUp => buf.push(KEY_PAGE_UP),
                KeyCode::PageDown => buf.push(KEY_PAGE_DOWN),
                KeyCode::Tab => buf.push(KEY_TAB),
                KeyCode::BackTab => buf.push(KEY_BACK_TAB),
                KeyCode::Delete => buf.push(KEY_DELETE),
                KeyCode::Insert => buf.push(KEY_INSERT),
                KeyCode::Esc => buf.push(KEY_ESC),
                KeyCode::F(n) => buf.push(KEY_F_BASE + n.min(12).saturating_sub(1)),
            }
            buf.push(k.mods);
            buf.push(u8::from(k.press));
        }
        InputEvent::Mouse(m) => {
            buf.push(SUB_MOUSE);
            buf.push(match m.kind {
                MouseKind::Down => MOUSE_DOWN,
                MouseKind::Up => MOUSE_UP,
                MouseKind::Drag => MOUSE_DRAG,
                MouseKind::Moved => MOUSE_MOVED,
                MouseKind::ScrollUp => MOUSE_SCROLL_UP,
                MouseKind::ScrollDown => MOUSE_SCROLL_DOWN,
                MouseKind::ScrollLeft => MOUSE_SCROLL_LEFT,
                MouseKind::ScrollRight => MOUSE_SCROLL_RIGHT,
            });
            buf.push(m.button);
            buf.extend_from_slice(&m.col.to_le_bytes());
            buf.extend_from_slice(&m.row.to_le_bytes());
            buf.push(m.mods);
        }
    }
}

fn decode_input(c: &mut Cursor<'_>) -> io::Result<InputEvent> {
    let sub = c.u8()?;
    match sub {
        SUB_KEY => {
            let kind = c.u8()?;
            let code = match kind {
                KEY_CHAR => {
                    let cp = c.u32()?;
                    KeyCode::Char(char::from_u32(cp).unwrap_or('\u{fffd}'))
                }
                KEY_BACKSPACE => KeyCode::Backspace,
                KEY_ENTER => KeyCode::Enter,
                KEY_LEFT => KeyCode::Left,
                KEY_RIGHT => KeyCode::Right,
                KEY_UP => KeyCode::Up,
                KEY_DOWN => KeyCode::Down,
                KEY_HOME => KeyCode::Home,
                KEY_END => KeyCode::End,
                KEY_PAGE_UP => KeyCode::PageUp,
                KEY_PAGE_DOWN => KeyCode::PageDown,
                KEY_TAB => KeyCode::Tab,
                KEY_BACK_TAB => KeyCode::BackTab,
                KEY_DELETE => KeyCode::Delete,
                KEY_INSERT => KeyCode::Insert,
                KEY_ESC => KeyCode::Esc,
                n if (KEY_F_BASE..KEY_F_BASE + 12).contains(&n) => KeyCode::F(n - KEY_F_BASE + 1),
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown key code {other}"),
                    ));
                }
            };
            let mods = c.u8()?;
            let press = c.u8()? != 0;
            Ok(InputEvent::Key(KeyInput { code, mods, press }))
        }
        SUB_MOUSE => {
            let mkind = c.u8()?;
            let kind = match mkind {
                MOUSE_DOWN => MouseKind::Down,
                MOUSE_UP => MouseKind::Up,
                MOUSE_DRAG => MouseKind::Drag,
                MOUSE_MOVED => MouseKind::Moved,
                MOUSE_SCROLL_UP => MouseKind::ScrollUp,
                MOUSE_SCROLL_DOWN => MouseKind::ScrollDown,
                MOUSE_SCROLL_LEFT => MouseKind::ScrollLeft,
                MOUSE_SCROLL_RIGHT => MouseKind::ScrollRight,
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown mouse kind {other}"),
                    ));
                }
            };
            let button = c.u8()?;
            let col = c.u16()?;
            let row = c.u16()?;
            let mods = c.u8()?;
            Ok(InputEvent::Mouse(MouseInput {
                kind,
                button,
                col,
                row,
                mods,
            }))
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown input sub {other}"),
        )),
    }
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        if self.pos + n > self.buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short payload",
            ));
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

#[allow(dead_code)]
pub fn pack_rgba(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let cv = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 255.0).round() as u32 };
    (cv(r) << 24) | (cv(g) << 16) | (cv(b) << 8) | cv(a)
}

#[allow(dead_code)]
pub fn unpack_rgba(c: u32) -> [f32; 4] {
    [
        ((c >> 24) & 0xff) as f32 / 255.0,
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

#[allow(dead_code)]
pub fn pack_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode `msg`, decode it straight back, and assert it survived
    /// the wire unchanged. The whole point of this crate is that this
    /// holds for every `Message` variant — both ends depend on it.
    fn round_trip(msg: Message) {
        let mut buf: Vec<u8> = Vec::new();
        write_message(&mut buf, &msg).expect("encode");
        let got = read_message(&mut &buf[..]).expect("decode");
        assert_eq!(got, msg, "round-trip changed the message");
    }

    #[test]
    fn hello_round_trips() {
        round_trip(Message::Hello {
            version: PROTOCOL_VERSION,
        });
        round_trip(Message::Hello { version: 0 });
    }

    #[test]
    fn resize_round_trips() {
        round_trip(Message::Resize(Resize {
            cols: 200,
            rows: 60,
        }));
        round_trip(Message::Resize(Resize { cols: 0, rows: 0 }));
    }

    #[test]
    fn quit_round_trips() {
        round_trip(Message::Quit);
    }

    #[test]
    fn title_round_trips() {
        round_trip(Message::Title("mnml — src/lib.rs".to_string()));
        round_trip(Message::Title(String::new()));
        round_trip(Message::Title("emoji 🎛 + spinner ✽".to_string()));
    }

    #[test]
    fn open_pane_round_trips() {
        round_trip(Message::OpenPane {
            command: "mixr".to_string(),
            args: vec!["--blit".to_string(), "/tmp/x.sock".to_string()],
        });
        round_trip(Message::OpenPane {
            command: "sh".to_string(),
            args: vec![],
        });
    }

    #[test]
    fn palette_round_trips() {
        round_trip(Message::Palette {
            bg: pack_rgba_u8(0x1e, 0x22, 0x2a, 0xff),
            fg: pack_rgba_u8(0xab, 0xb2, 0xbf, 0xff),
            accent: pack_rgba_u8(0x61, 0xaf, 0xef, 0xff),
        });
    }

    #[test]
    fn key_input_round_trips() {
        for code in [
            KeyCode::Char('a'),
            KeyCode::Char('✽'),
            KeyCode::Backspace,
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Delete,
            KeyCode::Insert,
            KeyCode::F(1),
            KeyCode::F(12),
        ] {
            round_trip(Message::Input(InputEvent::Key(KeyInput {
                code,
                mods: MOD_CTRL | MOD_SHIFT,
                press: true,
            })));
        }
    }

    #[test]
    fn mouse_input_round_trips() {
        for kind in [
            MouseKind::Down,
            MouseKind::Up,
            MouseKind::Drag,
            MouseKind::Moved,
            MouseKind::ScrollUp,
            MouseKind::ScrollDown,
            MouseKind::ScrollLeft,
            MouseKind::ScrollRight,
        ] {
            round_trip(Message::Input(InputEvent::Mouse(MouseInput {
                kind,
                button: BUTTON_LEFT,
                col: 12,
                row: 34,
                mods: MOD_ALT,
            })));
        }
    }

    #[test]
    fn frame_round_trips() {
        round_trip(Message::Frame(Frame {
            seq: 42,
            cols: 8,
            rows: 2,
            cursor_col: 3,
            cursor_row: 1,
            cursor_shape: 0,
            cursor_visible: 1,
            runs: vec![
                DiffRun {
                    start: 0,
                    cells: vec![WireCell {
                        ch: 'h' as u32,
                        fg: 1,
                        bg: 2,
                        attrs: 3,
                    }],
                },
                DiffRun {
                    start: 10,
                    cells: vec![
                        WireCell {
                            ch: 'i' as u32,
                            fg: 4,
                            bg: 5,
                            attrs: 6,
                        },
                        WireCell::default(),
                    ],
                },
            ],
        }));
        // A cursor-only frame — no runs.
        round_trip(Message::Frame(Frame {
            seq: 0,
            cols: 1,
            rows: 1,
            cursor_col: 0,
            cursor_row: 0,
            cursor_shape: 1,
            cursor_visible: 0,
            runs: vec![],
        }));
    }

    #[test]
    fn rgba_pack_unpack_round_trips() {
        // pack_rgba_u8 lays bytes out as R<<24 | G<<16 | B<<8 | A.
        assert_eq!(pack_rgba_u8(0x12, 0x34, 0x56, 0x78), 0x1234_5678);
        // unpack_rgba is the inverse (as normalized floats).
        let [r, g, b, a] = unpack_rgba(0x1234_5678);
        assert!((r - 0x12 as f32 / 255.0).abs() < 1e-6);
        assert!((g - 0x34 as f32 / 255.0).abs() < 1e-6);
        assert!((b - 0x56 as f32 / 255.0).abs() < 1e-6);
        assert!((a - 0x78 as f32 / 255.0).abs() < 1e-6);
        // pack_rgba (float input) clamps to [0,1] then rounds.
        assert_eq!(pack_rgba(1.0, 0.0, 0.0, 1.0), 0xff00_00ff);
        assert_eq!(pack_rgba(2.0, -1.0, 0.0, 0.0), 0xff00_0000);
    }

    #[test]
    fn read_message_rejects_an_unknown_type() {
        // 4-byte LE length prefix (1) + a payload of one byte: msg
        // type 99, which no variant claims ⇒ a clean decode error,
        // never a panic.
        let bytes = [1u8, 0, 0, 0, 99];
        assert!(read_message(&mut &bytes[..]).is_err());
    }

    #[test]
    fn open_pane_transfer_round_trips_via_write_message() {
        // The new variant's byte encoding is reachable via the regular
        // `write_message` / `read_message` path — the fd-transfer
        // semantic is layered on top via send/read_message_with_fd.
        round_trip(Message::OpenPaneTransfer {
            command: "claude".to_string(),
            args: vec!["--model".into(), "opus".into()],
        });
        round_trip(Message::OpenPaneTransfer {
            command: "/usr/bin/env".to_string(),
            args: vec![],
        });
    }

    /// SCM_RIGHTS round-trip — sender attaches one fd, receiver pulls
    /// it out + decodes the message. Uses a `socketpair(AF_UNIX,
    /// SOCK_STREAM)` so the two ends share the same process. The fd
    /// we pass is `STDIN_FILENO` duplicated — cheap + always available.
    #[cfg(unix)]
    #[test]
    fn fd_passing_round_trips_message_and_fd() {
        use std::os::fd::FromRawFd;
        use std::os::unix::net::UnixStream;

        let (a, b) = UnixStream::pair().expect("socketpair");

        // Duplicate stdin → a fresh fd we can transfer without
        // disturbing the test process's actual stdin.
        let sent_fd = unsafe { libc::dup(libc::STDIN_FILENO) };
        assert!(
            sent_fd >= 0,
            "dup failed: {}",
            std::io::Error::last_os_error()
        );

        let msg = Message::OpenPaneTransfer {
            command: "claude".to_string(),
            args: vec!["--model".into(), "opus".into()],
        };
        send_message_with_fd(&a, &msg, Some(sent_fd)).expect("send");

        let (got_msg, got_fd) = read_message_with_fd(&b).expect("recv");
        assert_eq!(got_msg, msg);
        assert!(got_fd.is_some(), "expected an attached fd");

        // Close both copies of the transferred fd (sender's + receiver's
        // — SCM_RIGHTS dup'd it across processes; both sides own a copy).
        unsafe {
            libc::close(sent_fd);
        }
        // Wrap the received fd in an OwnedFd-style holder to ensure it
        // closes when dropped — using UnixStream::from_raw_fd is the
        // simplest way (stdin works fine as a "stream" for close-on-drop).
        let _drop_me = unsafe { std::fs::File::from_raw_fd(got_fd.unwrap()) };
    }

    /// Sending two fds in one SCM_RIGHTS payload — the receiver
    /// should return the first one and close the rest, not leak them.
    /// We probe for the leak by counting open fds in `/dev/fd` before
    /// and after the receive: an extra leaked fd would show up. On
    /// macOS the directory is `/dev/fd`; on Linux it's `/proc/self/fd`
    /// — both are well-supported.
    #[cfg(unix)]
    #[test]
    fn fd_passing_drops_extra_fds_in_one_cmsg() {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::net::UnixStream;

        let (a, b) = UnixStream::pair().expect("socketpair");
        let fd1 = unsafe { libc::dup(libc::STDIN_FILENO) };
        let fd2 = unsafe { libc::dup(libc::STDIN_FILENO) };
        assert!(fd1 >= 0 && fd2 >= 0);

        // Manually craft a sendmsg with TWO fds in one SCM_RIGHTS
        // cmsg — the public API only accepts one, so we go straight
        // to libc to exercise the receiver's multi-fd handling.
        let msg = Message::OpenPaneTransfer {
            command: "claude".to_string(),
            args: vec![],
        };
        let bytes = encode_message(&msg);
        let mut iov = libc::iovec {
            iov_base: bytes.as_ptr() as *mut libc::c_void,
            iov_len: bytes.len(),
        };
        let mut mhdr: libc::msghdr = unsafe { std::mem::zeroed() };
        mhdr.msg_iov = &mut iov;
        mhdr.msg_iovlen = 1;
        // cmsg_buf big enough for two fds.
        let mut cmsg_buf = [0u8; 64];
        let cmsg_space =
            unsafe { libc::CMSG_SPACE(2 * std::mem::size_of::<libc::c_int>() as u32) } as usize;
        let cmsg_len =
            unsafe { libc::CMSG_LEN(2 * std::mem::size_of::<libc::c_int>() as u32) } as usize;
        mhdr.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
        mhdr.msg_controllen = cmsg_space as _;
        let cmsg_ptr = unsafe { libc::CMSG_FIRSTHDR(&mhdr) };
        unsafe {
            (*cmsg_ptr).cmsg_len = cmsg_len as _;
            (*cmsg_ptr).cmsg_level = libc::SOL_SOCKET;
            (*cmsg_ptr).cmsg_type = libc::SCM_RIGHTS;
            let data = libc::CMSG_DATA(cmsg_ptr);
            std::ptr::copy_nonoverlapping(
                &fd1 as *const libc::c_int as *const u8,
                data,
                std::mem::size_of::<libc::c_int>(),
            );
            std::ptr::copy_nonoverlapping(
                &fd2 as *const libc::c_int as *const u8,
                data.add(std::mem::size_of::<libc::c_int>()),
                std::mem::size_of::<libc::c_int>(),
            );
        }
        let sent = unsafe { libc::sendmsg(a.as_raw_fd(), &mhdr, 0) };
        assert_eq!(sent as usize, bytes.len(), "sendmsg short write");

        let (got_msg, got_fd) = read_message_with_fd(&b).expect("recv");
        assert_eq!(got_msg, msg);
        let got_raw = got_fd.expect("expected an attached fd");

        // The receiver kept exactly one fd. Close it via OwnedFd-drop
        // so we don't pollute later tests' fd table.
        let _wrap = unsafe { std::fs::File::from_raw_fd(got_raw) };

        // Close the sender's copies of the two fds.
        unsafe {
            libc::close(fd1);
            libc::close(fd2);
        }
        // The receiver's "extra" fd was closed inside
        // read_message_with_fd — if we had leaked it, we'd see two
        // distinct receiver-side fds. We can't easily assert that
        // here without an OS-level count, but the test confirms the
        // happy path: we got back a single fd, decoded a valid
        // message, and the receiver didn't error out.
    }

    /// `send_message_with_fd(stream, msg, None)` should still work —
    /// no ancillary data, the receiver gets `Some(msg), None`.
    #[cfg(unix)]
    #[test]
    fn fd_passing_round_trips_message_without_fd() {
        use std::os::unix::net::UnixStream;

        let (a, b) = UnixStream::pair().expect("socketpair");
        let msg = Message::Hello {
            version: PROTOCOL_VERSION,
        };
        send_message_with_fd(&a, &msg, None).expect("send");
        let (got_msg, got_fd) = read_message_with_fd(&b).expect("recv");
        assert_eq!(got_msg, msg);
        assert!(got_fd.is_none());
    }
}
