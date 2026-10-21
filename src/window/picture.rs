//! Pictures for exporting the fort (`ui/export.rs`): tiles painted the way
//! the window paints them, with the system's font when there is one, and
//! the PNG encoder the screenshots use.

use super::backend::TileBackend;
use super::glyphs::Fonts;
use ratatui::backend::Backend;
use ratatui::buffer::Cell;
use ratatui::style::Color;
use std::rc::Rc;

thread_local! {
    /// The fonts, loaded on first use (None when the system has none).
    static FONTS: std::cell::RefCell<Option<Option<Rc<Fonts>>>> = const { std::cell::RefCell::new(None) };
}

fn fonts() -> Option<Rc<Fonts>> {
    FONTS.with(|f| f.borrow_mut().get_or_insert_with(|| Fonts::load(None).ok().map(Rc::new)).clone())
}

/// A tile as glyph, ink and paper.
pub type Look = (char, (u8, u8, u8), (u8, u8, u8));

/// `w` x `h` tiles (row by row) painted as the window paints them, `cell`
/// pixels square (8 to 32): the pixels (0x00RRGGBB) and the picture's width and
/// height. Without a system font, letters are drawn as small blocks.
pub fn paint(cells: &[Look], w: usize, h: usize, cell: usize) -> (Vec<u32>, usize, usize) {
    let font: Box<dyn Fn(char, usize) -> Vec<u8>> = match fonts() {
        Some(f) => Box::new(move |c, s| f.raster(c, s)),
        None => Box::new(|_, s| {
            let mut m = vec![0; s * s];
            for y in s / 4..s * 3 / 4 {
                for x in s / 4..s * 3 / 4 {
                    m[y * s + x] = 255;
                }
            }
            m
        }),
    };
    let mut b = TileBackend::new(w * cell, h * cell, cell, font);
    let rgb = |c: (u8, u8, u8)| Color::Rgb(c.0, c.1, c.2);
    let painted: Vec<(u16, u16, Cell)> = cells
        .iter()
        .take(w * h)
        .enumerate()
        .map(|(i, &(ch, ink, paper))| {
            let mut c = Cell::default();
            c.set_char(ch).set_fg(rgb(ink)).set_bg(rgb(paper));
            ((i % w) as u16, (i / w) as u16, c)
        })
        .collect();
    let _ = b.draw(painted.iter().map(|(x, y, c)| (*x, *y, c)));
    let (pw, ph) = (b.width, b.height);
    (std::mem::take(&mut b.pixels), pw, ph)
}

/// A PNG of `pixels`, `w` a row.
pub fn png(pixels: &[u32], w: usize, h: usize) -> Vec<u8> {
    super::png::encode(pixels, w, h)
}
