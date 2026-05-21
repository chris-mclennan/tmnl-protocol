//! Binary wire format between the tmnl terminal and a backing app.
//!
//! Both the tmnl renderer and the mnml editor's `blit` backend depend on
//! this crate so the protocol is defined exactly once.

use std::io::{self, Read, Write};

pub const PROTOCOL_VERSION: u32 = 3;
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

#[derive(Clone, Debug)]
pub struct DiffRun {
    pub start: u32,
    pub cells: Vec<WireCell>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct Resize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct MouseInput {
    pub kind: MouseKind,
    pub button: u8,
    pub col: u16,
    pub row: u16,
    pub mods: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    Key(KeyInput),
    Mouse(MouseInput),
}

#[derive(Clone, Debug)]
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
            let cmd_len = c.u32()? as usize;
            if cmd_len > 64 * 1024 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("absurd command len {cmd_len}"),
                ));
            }
            let command = String::from_utf8(c.take(cmd_len)?.to_vec()).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("bad utf-8 command: {e}"))
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
            Ok(Message::OpenPane { command, args })
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

    #[test]
    fn palette_message_round_trips() {
        let msg = Message::Palette {
            bg: pack_rgba_u8(0x1e, 0x22, 0x2a, 0xff),
            fg: pack_rgba_u8(0xab, 0xb2, 0xbf, 0xff),
            accent: pack_rgba_u8(0x61, 0xaf, 0xef, 0xff),
        };
        let mut buf: Vec<u8> = Vec::new();
        write_message(&mut buf, &msg).unwrap();
        match read_message(&mut &buf[..]).unwrap() {
            Message::Palette { bg, fg, accent } => {
                assert_eq!(bg, pack_rgba_u8(0x1e, 0x22, 0x2a, 0xff));
                assert_eq!(fg, pack_rgba_u8(0xab, 0xb2, 0xbf, 0xff));
                assert_eq!(accent, pack_rgba_u8(0x61, 0xaf, 0xef, 0xff));
            }
            other => panic!("expected Palette, got {other:?}"),
        }
    }
}
