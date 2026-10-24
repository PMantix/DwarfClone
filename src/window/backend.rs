//! A ratatui backend that paints the cell grid into a pixel buffer of square
//! cells. ratatui hands `draw` only the cells that changed since the last
//! frame, so only those are painted again.

use super::glyphs::GlyphCache;
use super::tiles::Mask;
use crate::ui::adventure::{BigMap, BIG_MAP, HOLE};
use ratatui::backend::{Backend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier};
use std::io;

/// Cell sizes, in pixels, the zoom keys step between.
pub const MIN_CELL: usize = 8;
pub const MAX_CELL: usize = 32;
/// The largest the adventure map's tiles can be zoomed to.
pub const MAX_TILE: usize = 96;

/// The adventure map's tiles for a window of `cell`-pixel text: two thirds
/// as big again, an even number of pixels.
pub fn default_tile(cell: usize) -> usize {
    (cell * 5 / 3 / 2 * 2).min(MAX_TILE)
}

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
    /// The size the adventure map's tiles are wanted, in pixels.
    pub tile: usize,
    /// The adventure map as last painted big, and its glyphs.
    big: Option<BigMap>,
    big_cache: GlyphCache,
    /// The cells that are holes for the big map to show through, row by row.
    holes: Vec<bool>,
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
            tile: default_tile(cell.clamp(MIN_CELL, MAX_CELL)),
            big: None,
            big_cache: GlyphCache::new(MAX_TILE),
            holes: Vec::new(),
        }
    }

    /// Whether the adventure map was drawn big last frame (so the zoom
    /// keys zoom its tiles rather than the text).
    pub fn showing_big(&self) -> bool {
        self.big.is_some()
    }

    pub fn cell(&self) -> usize {
        self.cache.size()
    }

    /// Change the cell size; the caller clears the terminal so it all redraws.
    pub fn set_cell(&mut self, cell: usize) {
        self.cache.resize(cell.clamp(MIN_CELL, MAX_CELL));
    }

    /// Draw text with `font` from now on (a new face, or the same drawn
    /// wider or heavier): every glyph is forgotten, and the caller clears
    /// the terminal so it all redraws.
    pub fn set_font(&mut self, font: Box<dyn Fn(char, usize) -> Mask>) {
        self.font = font;
        self.cache.forget();
        self.big_cache.forget();
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
        self.blit(px, py, s, [px, py, px + s, py + s], cell, false);
    }

    /// Paint `cell` as a square `s` pixels across with its top left at
    /// (`px`, `py`), only the pixels inside `clip` (x0, y0, x1, y1); `big`
    /// takes its glyph from the big map's cache.
    fn blit(&mut self, px: usize, py: usize, s: usize, clip: [usize; 4], cell: &Cell, big: bool) {
        let x0 = clip[0].max(px);
        let y0 = clip[1].max(py);
        let x1 = clip[2].min(px + s).min(self.width);
        let y1 = clip[3].min(py + s).min(self.height);
        if x0 >= x1 || y0 >= y1 {
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
            for y in y0..y1 {
                self.pixels[y * w + x0..y * w + x1].fill(bg);
            }
        } else {
            let font = &self.font;
            let cache = if big { &mut self.big_cache } else { &mut self.cache };
            let mask = cache.get(ch, &**font);
            for y in y0..y1 {
                let row = (y - py) * s;
                let out = &mut self.pixels[y * w + x0..y * w + x1];
                for (o, &a) in out.iter_mut().zip(&mask[row + x0 - px..row + x1 - px]) {
                    *o = match a {
                        0 => bg,
                        255 => fg,
                        a => mix(bg, fg, a),
                    };
                }
            }
        }
        let under = py + s - 1 - s / 16;
        if m.contains(Modifier::UNDERLINED) && under >= y0 && under < y1 {
            self.pixels[under * w + x0..under * w + x1].fill(fg);
        }
        self.dirty = true;
    }

    /// Where the big map's tiles go, in pixels: the top left of the first
    /// and the size of one, the grid centred over the map's cells.
    fn big_origin(&self, b: &BigMap) -> (usize, usize, usize) {
        let s = self.cell();
        let t = b.tile as usize;
        let (aw, ah) = (b.area.width as usize * s, b.area.height as usize * s);
        let gx = b.area.x as usize * s + aw.saturating_sub(b.cols as usize * t) / 2;
        let gy = b.area.y as usize * s + ah.saturating_sub(b.rows as usize * t) / 2;
        (gx, gy, t)
    }

    /// Paint the hole at cell (x, y): black, and the big tiles it shows.
    fn paint_hole(&mut self, b: &BigMap, x: u16, y: u16) {
        let s = self.cell();
        let (px, py) = (x as usize * s, y as usize * s);
        if px + s > self.width || py + s > self.height {
            return;
        }
        let clip = [px, py, px + s, py + s];
        self.blit(px, py, s, clip, &Cell::default(), false);
        let (gx, gy, t) = self.big_origin(b);
        if t == 0 {
            return;
        }
        // (The tiles the cell overlaps, along one axis.)
        let span = |p: usize, g: usize, n: u16| -> std::ops::Range<usize> {
            let lo = p.saturating_sub(g) / t;
            let hi = ((p + s).saturating_sub(g) + t - 1) / t;
            lo.min(n as usize)..hi.min(n as usize)
        };
        for j in span(py, gy, b.rows) {
            for i in span(px, gx, b.cols) {
                let c = &b.cells[j * b.cols as usize + i];
                self.blit(gx + i * t, gy + j * t, t, clip, c, true);
            }
        }
    }

    /// Paint big tile `i` wherever a hole shows it.
    fn paint_tile(&mut self, b: &BigMap, i: usize) {
        let s = self.cell();
        let (gx, gy, t) = self.big_origin(b);
        let (px, py) = (gx + i % b.cols as usize * t, gy + i / b.cols as usize * t);
        let cols = self.grid().0 as usize;
        for cy in py / s..(py + t).div_ceil(s) {
            for cx in px / s..((px + t).div_ceil(s)).min(cols) {
                if self.holes.get(cy * cols + cx) == Some(&true) {
                    let clip = [cx * s, cy * s, cx * s + s, cy * s + s];
                    self.blit(px, py, t, clip, &b.cells[i], true);
                }
            }
        }
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
        // The adventure map drawn big this frame, if it was: its tiles show
        // through the cells left as holes, and only there, so whatever is
        // drawn over the map stays on top.
        let big = BIG_MAP.with(|b| b.borrow_mut().take());
        let old = self.big.take();
        let (cols, rows) = self.grid();
        let n = cols as usize * rows as usize;
        if self.holes.len() != n {
            self.holes = vec![false; n];
        }
        let mut opened = Vec::new();
        for (x, y, cell) in content {
            let hole = big.is_some() && cell.symbol().starts_with(HOLE);
            if let Some(h) = self.holes.get_mut(y as usize * cols as usize + x as usize) {
                *h = hole;
            }
            if hole {
                opened.push((x, y));
            } else {
                self.paint(x, y, cell);
            }
        }
        let Some(b) = big else { return Ok(()) };
        self.big_cache.resize(b.tile as usize);
        match old.filter(|o| (o.area, o.cols, o.rows, o.tile) == (b.area, b.cols, b.rows, b.tile)) {
            // The same map: the tiles that changed, and the new holes.
            Some(o) => {
                for i in 0..b.cells.len() {
                    if b.cells[i] != o.cells[i] {
                        self.paint_tile(&b, i);
                    }
                }
                for (x, y) in opened {
                    self.paint_hole(&b, x, y);
                }
            }
            // A new map, or zoomed: every hole.
            None => {
                for i in 0..n {
                    if self.holes[i] {
                        self.paint_hole(&b, (i % cols as usize) as u16, (i / cols as usize) as u16);
                    }
                }
            }
        }
        self.big = Some(b);
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
        self.big = None;
        self.holes.clear();
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
    fn a_new_font_forgets_the_glyphs_and_redraws() {
        let mut term = Terminal::new(backend(4, 1)).unwrap();
        let text = |f: &mut ratatui::Frame| f.render_widget(ratatui::widgets::Paragraph::new("ab"), f.size());
        term.draw(text).unwrap();
        assert_eq!(term.backend().glyphs_drawn(), 2);
        let white = rgb(Color::Reset, true, false);
        assert_eq!(term.backend().pixels[8 * 64 + 8], white, "the old font fills the middle");
        assert_eq!(term.backend().pixels[64 + 1], 0);
        // A font that fills the whole cell.
        term.backend_mut().set_font(Box::new(|_, s| vec![255; s * s]));
        term.clear().unwrap();
        term.draw(text).unwrap();
        assert_eq!(term.backend().glyphs_drawn(), 4, "both letters drawn again in the new font");
        assert_eq!(term.backend().pixels[64 + 1], white, "the new font is on screen");
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

    #[test]
    fn the_big_map_shows_through_its_holes_and_under_what_is_drawn_over_it() {
        use crate::ui::adventure::{BigMap, BIG_MAP, HOLE};
        // A 4 x 4 cell map of 16 px cells, holding 2 x 2 tiles of 32 px.
        let mut term = Terminal::new(backend(4, 4)).unwrap();
        let paint = |red: u8, over: bool| {
            move |f: &mut ratatui::Frame| {
                let buf = f.buffer_mut();
                for y in 0..4 {
                    for x in 0..4 {
                        buf.get_mut(x, y).set_char(HOLE);
                    }
                }
                if over {
                    buf.get_mut(3, 3).set_char(' ').set_bg(Color::Rgb(0, 0, 200));
                }
                let mut cells = vec![Cell::default(); 4];
                cells[3].set_char(' ').set_bg(Color::Rgb(red, 0, 0));
                BIG_MAP.with(|b| *b.borrow_mut() = Some(BigMap { area: Rect::new(0, 0, 4, 4), cols: 2, rows: 2, tile: 32, cells }));
            }
        };
        term.draw(paint(200, false)).unwrap();
        let px = |b: &TileBackend, x: usize, y: usize| b.pixels[y * b.width + x];
        assert_eq!(px(term.backend(), 40, 40), pack(200, 0, 0), "the bottom right tile covers four cells");
        assert_eq!(px(term.backend(), 60, 60), pack(200, 0, 0));
        assert_eq!(px(term.backend(), 10, 10), 0);
        // Only the tile changed: it's repainted though no cell changed.
        term.draw(paint(100, false)).unwrap();
        assert_eq!(px(term.backend(), 40, 40), pack(100, 0, 0));
        // Something drawn over the map stays on top of it.
        term.draw(paint(150, true)).unwrap();
        assert_eq!(px(term.backend(), 60, 60), pack(0, 0, 200));
        assert_eq!(px(term.backend(), 40, 40), pack(150, 0, 0));
        assert!(term.backend().showing_big());
    }
}
