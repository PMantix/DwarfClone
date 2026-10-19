//! The window's keyboard and mouse, turned into the crossterm events the game
//! already reads, so `App::on_key` and `App::on_mouse` can't tell the window
//! from a terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// The held modifiers as crossterm has them.
pub fn modifiers(m: ModifiersState) -> KeyModifiers {
    let mut k = KeyModifiers::NONE;
    if m.shift_key() {
        k |= KeyModifiers::SHIFT;
    }
    if m.control_key() {
        k |= KeyModifiers::CONTROL;
    }
    if m.alt_key() {
        k |= KeyModifiers::ALT;
    }
    k
}

/// A key pressed in the window as a terminal would report it, or None for
/// keys the game has no use for (a modifier on its own, media keys...).
pub fn key_event(key: &Key, mods: ModifiersState) -> Option<KeyEvent> {
    let mut m = modifiers(mods);
    let code = match key {
        Key::Named(n) => match n {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Escape => KeyCode::Esc,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Tab if mods.shift_key() => KeyCode::BackTab,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Space => KeyCode::Char(' '),
            NamedKey::ArrowUp => KeyCode::Up,
            NamedKey::ArrowDown => KeyCode::Down,
            NamedKey::ArrowLeft => KeyCode::Left,
            NamedKey::ArrowRight => KeyCode::Right,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::F1 => KeyCode::F(1),
            NamedKey::F2 => KeyCode::F(2),
            NamedKey::F3 => KeyCode::F(3),
            NamedKey::F4 => KeyCode::F(4),
            NamedKey::F5 => KeyCode::F(5),
            NamedKey::F6 => KeyCode::F(6),
            NamedKey::F7 => KeyCode::F(7),
            NamedKey::F8 => KeyCode::F(8),
            NamedKey::F9 => KeyCode::F(9),
            NamedKey::F10 => KeyCode::F(10),
            NamedKey::F11 => KeyCode::F(11),
            NamedKey::F12 => KeyCode::F(12),
            _ => return None,
        },
        Key::Character(s) => {
            let mut c = s.chars().next()?;
            // Ctrl+letter may come as the old control code (Ctrl+A is 1).
            if (c as u32) < 0x20 {
                c = char::from_u32(c as u32 + 0x60)?;
            }
            if mods.control_key() && mods.alt_key() {
                // AltGr (Ctrl+Alt to Windows) typing @, € or { on many layouts.
                m.remove(KeyModifiers::CONTROL | KeyModifiers::ALT);
            } else if mods.control_key() || mods.alt_key() {
                // Ctrl+Z, not Ctrl+Shift+z: the letter as typed without shift.
                c = c.to_ascii_lowercase();
            }
            KeyCode::Char(c)
        }
        _ => return None,
    };
    Some(KeyEvent::new(code, m))
}

/// Mouse state, turned into terminal-style events in cell coordinates.
#[derive(Default)]
pub struct Mouse {
    /// The pointer, in pixels, if it's over the window.
    pub pos: Option<(f64, f64)>,
    /// The cell last reported.
    cell: Option<(u16, u16)>,
    /// Left, right and middle held.
    held: [bool; 3],
}

fn button(i: usize) -> MouseButton {
    [MouseButton::Left, MouseButton::Right, MouseButton::Middle][i]
}

impl Mouse {
    fn at(&self, cell: usize) -> (u16, u16) {
        let (x, y) = self.pos.unwrap_or((0.0, 0.0));
        let c = |v: f64| (v.max(0.0) as usize / cell).min(u16::MAX as usize) as u16;
        (c(x), c(y))
    }

    fn event(&self, kind: MouseEventKind, cell: usize, m: KeyModifiers) -> MouseEvent {
        let (column, row) = self.at(cell);
        MouseEvent { kind, column, row, modifiers: m }
    }

    /// The pointer moved to (x, y) pixels: a drag with a button held, a move
    /// without, once per cell crossed.
    pub fn moved(&mut self, x: f64, y: f64, cell: usize, m: KeyModifiers) -> Option<MouseEvent> {
        self.pos = Some((x, y));
        let at = self.at(cell);
        if self.cell == Some(at) {
            return None;
        }
        self.cell = Some(at);
        let kind = match self.held.iter().position(|&h| h) {
            Some(i) => MouseEventKind::Drag(button(i)),
            None => MouseEventKind::Moved,
        };
        Some(self.event(kind, cell, m))
    }

    /// The pointer left the window.
    pub fn left(&mut self) {
        if !self.held.iter().any(|&h| h) {
            self.pos = None;
            self.cell = None;
        }
    }

    /// Button `i` (0 left, 1 right, 2 middle) pressed or released.
    pub fn button(&mut self, i: usize, down: bool, cell: usize, m: KeyModifiers) -> Option<MouseEvent> {
        if self.pos.is_none() || self.held[i] == down {
            self.held[i] = down;
            return None;
        }
        self.held[i] = down;
        let kind = if down { MouseEventKind::Down(button(i)) } else { MouseEventKind::Up(button(i)) };
        Some(self.event(kind, cell, m))
    }

    /// The wheel turned: up (away from you) is positive.
    pub fn wheel(&self, up: bool, cell: usize, m: KeyModifiers) -> Option<MouseEvent> {
        self.pos?;
        Some(self.event(if up { MouseEventKind::ScrollUp } else { MouseEventKind::ScrollDown }, cell, m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    fn key(k: &Key, mods: ModifiersState) -> (KeyCode, KeyModifiers) {
        let e = key_event(k, mods).unwrap();
        assert_eq!(e.kind, crossterm::event::KeyEventKind::Press);
        (e.code, e.modifiers)
    }

    #[test]
    fn letters_and_shifted_letters() {
        let none = ModifiersState::empty();
        assert_eq!(key(&ch("d"), none), (KeyCode::Char('d'), KeyModifiers::NONE));
        // Shift gives the capital, and says so like crossterm on Windows.
        assert_eq!(key(&ch("D"), ModifiersState::SHIFT), (KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(key(&ch("?"), ModifiersState::SHIFT), (KeyCode::Char('?'), KeyModifiers::SHIFT));
        assert_eq!(key(&Key::Named(NamedKey::Space), none), (KeyCode::Char(' '), KeyModifiers::NONE));
    }

    #[test]
    fn control_and_altgr() {
        assert_eq!(key(&ch("z"), ModifiersState::CONTROL), (KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(key(&ch("\u{3}"), ModifiersState::CONTROL), (KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(key(&ch("Z"), ModifiersState::CONTROL | ModifiersState::SHIFT), (KeyCode::Char('z'), KeyModifiers::CONTROL | KeyModifiers::SHIFT));
        assert_eq!(key(&ch("+"), ModifiersState::CONTROL), (KeyCode::Char('+'), KeyModifiers::CONTROL));
        // AltGr+Q on a German keyboard is @, typed plain.
        assert_eq!(key(&ch("@"), ModifiersState::CONTROL | ModifiersState::ALT), (KeyCode::Char('@'), KeyModifiers::NONE));
        assert_eq!(key(&ch("x"), ModifiersState::ALT), (KeyCode::Char('x'), KeyModifiers::ALT));
    }

    #[test]
    fn named_keys() {
        let none = ModifiersState::empty();
        let n = |k: NamedKey, m: ModifiersState| key(&Key::Named(k), m);
        assert_eq!(n(NamedKey::ArrowUp, none).0, KeyCode::Up);
        assert_eq!(n(NamedKey::ArrowLeft, ModifiersState::SHIFT), (KeyCode::Left, KeyModifiers::SHIFT));
        assert_eq!(n(NamedKey::F3, none).0, KeyCode::F(3));
        assert_eq!(n(NamedKey::F12, none).0, KeyCode::F(12));
        assert_eq!(n(NamedKey::Enter, none).0, KeyCode::Enter);
        assert_eq!(n(NamedKey::Escape, none).0, KeyCode::Esc);
        assert_eq!(n(NamedKey::Tab, none).0, KeyCode::Tab);
        assert_eq!(n(NamedKey::Tab, ModifiersState::SHIFT).0, KeyCode::BackTab);
        assert_eq!(n(NamedKey::Backspace, none).0, KeyCode::Backspace);
        assert_eq!(n(NamedKey::PageDown, none).0, KeyCode::PageDown);
        assert_eq!(n(NamedKey::Home, none).0, KeyCode::Home);
        assert_eq!(n(NamedKey::Delete, none).0, KeyCode::Delete);
        assert!(key_event(&Key::Named(NamedKey::Shift), ModifiersState::SHIFT).is_none());
    }

    #[test]
    fn mouse_in_cells() {
        let none = KeyModifiers::NONE;
        let mut m = Mouse::default();
        let kind = |e: Option<MouseEvent>| e.map(|e| (e.kind, e.column, e.row));
        assert_eq!(kind(m.moved(40.0, 20.0, 16, none)), Some((MouseEventKind::Moved, 2, 1)));
        // Within the same cell: nothing new.
        assert_eq!(kind(m.moved(45.0, 30.0, 16, none)), None);
        assert_eq!(kind(m.button(0, true, 16, none)), Some((MouseEventKind::Down(MouseButton::Left), 2, 1)));
        assert_eq!(kind(m.moved(70.0, 30.0, 16, none)), Some((MouseEventKind::Drag(MouseButton::Left), 4, 1)));
        assert_eq!(kind(m.button(0, false, 16, none)), Some((MouseEventKind::Up(MouseButton::Left), 4, 1)));
        assert_eq!(kind(m.button(1, true, 16, none)), Some((MouseEventKind::Down(MouseButton::Right), 4, 1)));
        assert_eq!(kind(m.wheel(true, 16, none)), Some((MouseEventKind::ScrollUp, 4, 1)));
        assert_eq!(kind(m.wheel(false, 8, none)), Some((MouseEventKind::ScrollDown, 8, 3)));
        m.button(1, false, 16, none);
        m.left();
        assert_eq!(kind(m.wheel(true, 16, none)), None, "not over the window");
    }
}
