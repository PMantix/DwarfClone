//! A ratatui backend that paints the cell grid into a pixel buffer of square
//! cells. ratatui hands `draw` only the cells that changed since the last
//! frame, so only those are painted again.

use super::glyphs::GlyphCache;
use super::tiles::Mask;
use ratatui::backend::{Backend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier};
use std::io;

/// Cell sizes, in pixels, the zoom keys step between.
pub const MIN_CELL: usize = 8;
pub const MAX_CELL: usize = 32;

pub struct TileBackend {
    /// The picture, 0x00RRGGBB a pixel, `width` pixels a row.
    pub pixels: Vec<u32>,
    pub width: usize,
    pub height: usize,
    /// The space there is to fill (the window's inside), which the buffer
    /// catches up with on the next clear.
    pub avail: (usize, usize),
    cache: GlyphCache,
    font: Box<dyn Fn(char, usize) -> Mask>,
    /// Something was painted since the picture was last shown.
    pub dirty: bool,
    cursor: (u16, u16),
}

impl TileBackend {
    /// A backend `w` x `h` pixels with `cell`-pixel cells, drawing text with `font`.
    pub fn new(w: usize, h: usize, cell: usize, font: Box<dyn Fn(char, usize) -> Mask>) -> TileBackend {
        TileBackend {
            pixels: vec![0; w * h],
            width: w,
            height: h,
            avail: (w, h),
            cache: GlyphCache::new(cell.clamp(MIN_CELL, MAX_CELL)),
            font,
            dirty: true,
            cursor: (0, 0),
        }
    }

    pub fn cell(&self) -> usize {
        self.cache.size()
    }

    /// Change the cell size; the caller clears the terminal so it all redraws.
    pub fn set_cell(&mut self, cell: usize) {
        self.cache.resize(cell.clamp(MIN_CELL, MAX_CELL));
    }

    /// How many glyphs have been drawn into the cache (for tests).
    #[cfg(test)]
    pub fn glyphs_drawn(&self) -> usize {
        self.cache.drawn
    }

    fn grid(&self) -> (u16, u16) {
        let s = self.cell();
        ((self.avail.0 / s).clamp(1, u16::MAX as usize) as u16, (self.avail.1 / s).clamp(1, u16::MAX as usize) as u16)
    }

    /// Paint one cell.
    fn paint(&mut self, x: u16, y: u16, cell: &Cell) {
        let s = self.cell();
        let (px, py) = (x as usize * s, y as usize * s);
        if px + s > self.width || py + s > self.height {
            return;
        }
        let m = cell.modifier;
        let mut fg = rgb(cell.fg, true, m.contains(Modifier::BOLD));
        let mut bg = rgb(cell.bg, false, false);
        if m.contains(Modifier::REVERSED) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if m.contains(Modifier::DIM) {
            fg = mix(bg, fg, 150);
        }
        if m.contains(Modifier::HIDDEN) {
            fg = bg;
        }
        let ch = cell.symbol().chars().next().unwrap_or(' ');
        let w = self.width;
        if ch == ' ' || fg == bg {
            for row in 0..s {
                let at = (py + row) * w + px;
                self.pixels[at..at + s].fill(bg);
            }
        } else {
            let font = &self.font;
            let mask = self.cache.get(ch, &**font);
            for row in 0..s {
                let at = (py + row) * w + px;
                let out = &mut self.pixels[at..at + s];
                for (o, &a) in out.iter_mut().zip(&mask[row * s..row * s + s]) {
                    *o = match a {
                        0 => bg,
                        255 => fg,
                        a => mix(bg, fg, a),
                    };
                }
            }
        }
        if m.contains(Modifier::UNDERLINED) {
            let at = (py + s - 1 - s / 16) * w + px;
            self.pixels[at..at + s].fill(fg);
        }
        self.dirty = true;
    }
}

/// `a` blended towards `b` by `t` of 255.
fn mix(a: u32, b: u32, t: u8) -> u32 {
    let t = t as u32;
    let ch = |shift: u32| {
        let (x, y) = ((a >> shift) & 0xff, (b >> shift) & 0xff);
        ((x * (255 - t) + y * t + 127) / 255) << shift
    };
    ch(16) | ch(8) | ch(0)
}

const fn pack(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// The sixteen terminal colours, as Windows Terminal shows them.
const PALETTE: [u32; 16] = [
    pack(12, 12, 12),
    pack(197, 15, 31),
    pack(19, 161, 14),
    pack(193, 156, 0),
    pack(0, 55, 218),
    pack(136, 23, 152),
    pack(58, 150, 221),
    pack(204, 204, 204),
    pack(118, 118, 118),
    pack(231, 72, 86),
    pack(22, 198, 12),
    pack(249, 241, 165),
    pack(59, 120, 255),
    pack(180, 0, 158),
    pack(97, 214, 214),
    pack(242, 242, 242),
];

/// A ratatui colour as a pixel. `Reset` is the default text colour in front
/// and black behind; bold brightens the eight dark named colours.
pub fn rgb(c: Color, fg: bool, bold: bool) -> u32 {
    let named = |i: usize| PALETTE[if bold && i < 8 { i + 8 } else { i }];
    match c {
        Color::Reset => {
            if fg {
                PALETTE[if bold { 15 } else { 7 }]
            } else {
                0
            }
        }
        Color::Black => named(0),
        Color::Red => named(1),
        Color::Green => named(2),
        Color::Yellow => named(3),
        Color::Blue => named(4),
        Color::Magenta => named(5),
        Color::Cyan => named(6),
        Color::Gray => named(7),
        Color::DarkGray => named(8),
        Color::LightRed => named(9),
        Color::LightGreen => named(10),
        Color::LightYellow => named(11),
        Color::LightBlue => named(12),
        Color::LightMagenta => named(13),
        Color::LightCyan => named(14),
        Color::White => named(15),
        Color::Rgb(r, g, b) => pack(r, g, b),
        Color::Indexed(i) => match i {
            0..=15 => named(i as usize),
            16..=231 => {
                // The 6x6x6 colour cube.
                let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                let i = i - 16;
                pack(level(i / 36), level(i / 6 % 6), level(i % 6))
            }
            _ => {
                let v = 8 + (i - 232) * 10;
                pack(v, v, v)
            }
        },
    }
}

impl Backend for TileBackend {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, cell) in content {
            self.paint(x, y, cell);
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        Ok(self.cursor)
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.cursor = (x, y);
        Ok(())
    }

    /// Clear to black, first fitting the buffer to the space there is now.
    fn clear(&mut self) -> io::Result<()> {
        let (w, h) = self.avail;
        if (w, h) != (self.width, self.height) {
            self.pixels = vec![0; w * h];
            (self.width, self.height) = (w, h);
        } else {
            self.pixels.fill(0);
        }
        self.dirty = true;
        Ok(())
    }

    fn size(&self) -> io::Result<Rect> {
        let (c, r) = self.grid();
        Ok(Rect::new(0, 0, c, r))
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        let (c, r) = self.grid();
        Ok(WindowSize { columns_rows: Size { width: c, height: r }, pixels: Size { width: self.width as u16, height: self.height as u16 } })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::style::Style;
    use ratatui::Terminal;

    fn backend(cols: usize, rows: usize) -> TileBackend {
        // A stand-in font: every letter a solid block in its middle.
        TileBackend::new(cols * 16, rows * 16, 16, Box::new(|_, s| {
            let mut m = vec![0; s * s];
            for y in s / 4..s * 3 / 4 {
                for x in s / 4..s * 3 / 4 {
                    m[y * s + x] = 255;
                }
            }
            m
        }))
    }

    #[test]
    fn a_box_drawn_by_ratatui_joins_up_in_pixels() {
        let mut term = Terminal::new(backend(6, 3)).unwrap();
        term.draw(|f| {
            let block = ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).style(Style::default().fg(Color::White));
            f.render_widget(block, f.size());
        })
        .unwrap();
        let b = term.backend();
        let white = rgb(Color::White, true, false);
        // The top border is one unbroken run of white from the first pixel
        // of the first cell to the last pixel of the last.
        let row = (0..16).find(|&y| b.pixels[y * b.width + 8] == white).expect("a lit row");
        assert!((8..b.width - 8).all(|x| b.pixels[row * b.width + x] == white), "the line has a gap");
        // The left border runs from the corner down to the bottom corner.
        let col = (0..16).find(|&x| b.pixels[24 * b.width + x] == white).expect("a lit column");
        assert!((row..b.height - 8).all(|y| b.pixels[y * b.width + col] == white));
    }

    #[test]
    fn only_changed_cells_are_painted_and_glyphs_are_cached() {
        let mut term = Terminal::new(backend(10, 2)).unwrap();
        let text = |t: &'static str| move |f: &mut ratatui::Frame| f.render_widget(ratatui::widgets::Paragraph::new(t), f.size());
        term.draw(text("aaaa")).unwrap();
        assert_eq!(term.backend().glyphs_drawn(), 1, "one glyph for four a's");
        term.backend_mut().dirty = false;
        term.draw(text("aaaa")).unwrap();
        assert!(!term.backend().dirty, "nothing changed, nothing painted");
        term.draw(text("aaab")).unwrap();
        assert!(term.backend().dirty);
        assert_eq!(term.backend().glyphs_drawn(), 2);
    }

    #[test]
    fn colours_come_from_the_cell() {
        let mut b = backend(2, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf.get_mut(0, 0).set_char('█').set_fg(Color::Rgb(10, 20, 30));
        buf.get_mut(1, 0).set_char(' ').set_bg(Color::Indexed(196));
        b.draw(buf.content().iter().enumerate().map(|(i, c)| (i as u16, 0, c))).unwrap();
        assert_eq!(b.pixels[5 * b.width + 5], pack(10, 20, 30));
        assert_eq!(b.pixels[5 * b.width + 20], pack(255, 0, 0));
        assert_eq!(rgb(Color::Red, true, true), rgb(Color::LightRed, true, false), "bold brightens");
        assert_eq!(rgb(Color::Indexed(232), true, false), pack(8, 8, 8));
    }

    #[test]
    fn the_grid_follows_the_window_and_the_zoom() {
        let mut b = backend(10, 5);
        assert_eq!(b.size().unwrap(), Rect::new(0, 0, 10, 5));
        b.avail = (330, 100);
        assert_eq!(b.size().unwrap(), Rect::new(0, 0, 20, 6));
        b.set_cell(32);
        assert_eq!(b.size().unwrap(), Rect::new(0, 0, 10, 3));
        b.clear().unwrap();
        assert_eq!((b.width, b.height, b.pixels.len()), (330, 100, 33000));
        b.set_cell(99);
        assert_eq!(b.cell(), MAX_CELL);
    }
}
