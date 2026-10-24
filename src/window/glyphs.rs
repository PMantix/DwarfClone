//! Text glyphs from the system's monospace font, scaled to fill a square
//! cell, and the cache of every glyph drawn so far. Nothing here ships a
//! font: the font files are read from the player's own system at startup.

use super::display::{Weight, Width};
use super::tiles::{self, Mask};
use std::collections::HashMap;
use std::path::PathBuf;

/// The monospace fonts tried in turn, then symbol fonts for characters the
/// first one lacks (☺, ♪, ⚔, braille...).
pub const MONO: [&str; 3] = ["CascadiaMono.ttf", "consola.ttf", "lucon.ttf"];
/// The same, by the names a player knows them by.
pub const MONO_NAMES: [&str; 3] = ["Cascadia Mono", "Consolas", "Lucida Console"];
const FALLBACK: [&str; 3] = ["seguisym.ttf", "consola.ttf", "segoeui.ttf"];

pub fn font_dir() -> std::path::PathBuf {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    std::path::Path::new(&windir).join("Fonts")
}

/// The loaded fonts, and how big to draw them to fill a cell.
pub struct Fonts {
    /// The text font's file.
    pub path: PathBuf,
    /// The text font first, then the fallbacks.
    fonts: Vec<fontdue::Font>,
    /// From the top of a capital to the bottom of a descender, per pixel of size.
    cap: f32,
    descent: f32,
    advance: f32,
}

impl Fonts {
    /// The font at `path`, or the first of the usual monospace fonts found.
    pub fn load(path: Option<&str>) -> Result<Fonts, String> {
        let dir = font_dir();
        let read = |p: &std::path::Path| -> Option<fontdue::Font> {
            let bytes = std::fs::read(p).ok()?;
            fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
        };
        let (main, path) = match path {
            Some(p) => (read(std::path::Path::new(p)).ok_or_else(|| format!("Couldn't read the font {} (give --font a .ttf or .otf file).", p))?, PathBuf::from(p)),
            None => MONO.iter().find_map(|f| Some((read(&dir.join(f))?, dir.join(f)))).ok_or_else(|| {
                format!("No monospace font found in {} (looked for {}). Give one with --font PATH.", dir.display(), MONO.join(", "))
            })?,
        };
        let mut fonts = vec![main];
        fonts.extend(FALLBACK.iter().filter_map(|f| read(&dir.join(f))));
        // Measure at a large size: a capital's top, a descender's bottom.
        let probe = 100.0;
        let top = fonts[0].metrics('H', probe);
        let low = fonts[0].metrics('g', probe);
        let cap = (top.ymin as f32 + top.height as f32) / probe;
        let descent = (-(low.ymin as f32) / probe).max(0.1);
        let advance = fonts[0].metrics('M', probe).advance_width / probe;
        Ok(Fonts { path, fonts, cap: cap.max(0.4), descent, advance: advance.max(0.3) })
    }

    /// `c` drawn to fill a cell `s` pixels square: capitals and descenders
    /// together take the cell's height, and letters are widened a little
    /// (never squeezed) and centred.
    pub fn raster(&self, c: char, s: usize) -> Mask {
        self.styled(c, s, Width::Normal, Weight::Normal)
    }

    /// `c` drawn to fill a cell `s` pixels square, its letters stretched
    /// `width`-wide and drawn `weight`-heavy.
    pub fn styled(&self, c: char, s: usize, width: Width, weight: Weight) -> Mask {
        let mut m = self.shape(c, s, width);
        weight.apply(&mut m, s);
        m
    }

    fn shape(&self, c: char, s: usize, width: Width) -> Mask {
        let font = self.fonts.iter().find(|f| f.lookup_glyph_index(c) != 0);
        let Some(font) = font else { return missing(s) };
        let sf = s as f32;
        let margin = (sf / 16.0).max(1.0);
        let px = (sf - margin) / (self.cap + self.descent);
        let baseline = margin / 2.0 + self.cap * px;
        let adv = self.advance * px;
        let (fill, most) = width.stretch();
        let stretch = (fill * sf / adv).clamp(1.0, most);
        let (m, bmp) = font.rasterize(c, px);
        let mut out = vec![0u8; s * s];
        if m.width == 0 || m.height == 0 {
            return out;
        }
        let pen = (sf - m.advance_width.max(adv * 0.5) * stretch) / 2.0;
        let top = (baseline - (m.ymin as f32 + m.height as f32)).round() as i32;
        for j in 0..m.height {
            let y = top + j as i32;
            if y < 0 || y >= s as i32 {
                continue;
            }
            let row = &bmp[j * m.width..(j + 1) * m.width];
            for x in 0..s {
                // Back from the cell's pixel to the glyph's, between two columns.
                let u = (x as f32 + 0.5 - pen) / stretch - m.xmin as f32 - 0.5;
                if u <= -1.0 || u >= m.width as f32 {
                    continue;
                }
                let i = u.floor();
                let f = u - i;
                let i = i as i32;
                let get = |k: i32| if k >= 0 && (k as usize) < m.width { row[k as usize] as f32 } else { 0.0 };
                let v = get(i) * (1.0 - f) + get(i + 1) * f;
                out[y as usize * s + x] = v.round().min(255.0) as u8;
            }
        }
        out
    }
}

/// A character no font has: an empty box.
fn missing(s: usize) -> Mask {
    let mut m = vec![0u8; s * s];
    let (a, b) = (s / 4, s - s / 4 - 1);
    for i in a..=b {
        for (x, y) in [(i, s / 6), (i, s - s / 6 - 1), (a, i.min(s - 1)), (b, i.min(s - 1))] {
            m[y * s + x] = 200;
        }
    }
    m
}

/// Every glyph drawn at the current cell size, as coverage masks: our own
/// tiles where we have one, the font for the rest. Colour is applied when a
/// cell is painted, so one mask serves every colour.
pub struct GlyphCache {
    size: usize,
    masks: HashMap<char, Mask>,
    /// How many glyphs have been drawn (tests check the cache is used).
    pub drawn: usize,
}

impl GlyphCache {
    pub fn new(size: usize) -> GlyphCache {
        GlyphCache { size, masks: HashMap::new(), drawn: 0 }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Forget every glyph (a new font, or the same drawn differently).
    pub fn forget(&mut self) {
        self.masks.clear();
    }

    /// A new cell size forgets every glyph.
    pub fn resize(&mut self, size: usize) {
        if size != self.size {
            self.size = size;
            self.masks.clear();
        }
    }

    /// The mask for `c`, drawing it with `font` the first time.
    pub fn get(&mut self, c: char, font: &dyn Fn(char, usize) -> Mask) -> &[u8] {
        let s = self.size;
        let drawn = &mut self.drawn;
        self.masks.entry(c).or_insert_with(|| {
            *drawn += 1;
            tiles::tile(c, s).unwrap_or_else(|| font(c, s))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_drawn_once_per_size() {
        let mut cache = GlyphCache::new(16);
        let font = |_: char, s: usize| vec![7u8; s * s];
        assert_eq!(cache.get('a', &font).len(), 256);
        cache.get('a', &font);
        cache.get('█', &font);
        cache.get('a', &font);
        assert_eq!(cache.drawn, 2);
        // Our own tiles come from the tileset, text from the font.
        assert_eq!(cache.get('a', &font)[0], 7);
        assert_eq!(cache.get('█', &font)[0], 255);
        // A new size starts again, at the new size.
        cache.resize(20);
        assert_eq!(cache.get('a', &font).len(), 400);
        assert_eq!(cache.drawn, 3);
        cache.resize(20);
        cache.get('a', &font);
        assert_eq!(cache.drawn, 3);
    }

    #[test]
    fn system_font_fills_the_cell_height() {
        // Only where the usual fonts are installed (any Windows PC).
        let Ok(fonts) = Fonts::load(None) else { return };
        let s = 16;
        let rows = |c: char| -> Vec<usize> {
            let m = fonts.raster(c, s);
            (0..s).filter(|&y| (0..s).any(|x| m[y * s + x] > 100)).collect()
        };
        let (h, g) = (rows('H'), rows('g'));
        // A capital's top near the top of the cell, a descender's foot near the bottom.
        assert!(*h.first().unwrap() <= 2, "H starts at row {:?}", h);
        assert!(*g.last().unwrap() >= s - 3, "g ends at row {:?}", g);
        // Letters sit inside the cell, centred.
        let m = fonts.raster('W', s);
        let cols: Vec<usize> = (0..s).filter(|&x| (0..s).any(|y| m[y * s + x] > 100)).collect();
        let (l, r) = (cols[0], s - 1 - cols[cols.len() - 1]);
        assert!(l.abs_diff(r) <= 2, "W is off centre: {} left, {} right", l, r);
    }
}
