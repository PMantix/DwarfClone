//! Export the fort (UI-10, `e` on the Highlights screen): a folder under
//! `exports/` to share the ruin. For every level the fort has walked on, an
//! overview picture in the style of the world map (`level-zN.png`, one
//! small block a tile, painted from the map as it draws) and the level as
//! text (`level-zN.txt`); the fort's best moment as a little film
//! (`best-moment.gif`); and its story, as a page (`fort.html`, no scripts)
//! and as plain text (`story.txt`): the chronicle year by year, the sagas,
//! the battles, the fallen and their last words, and the highlight reel.

use super::highlights::{best, capture};
use crate::app::App;
use crate::game::reel::{Clip, Look, CLIP_H, CLIP_W};
use crate::game::{Game, YEAR};
use crate::geom::Pos;
use std::path::{Path, PathBuf};

/// Pixels a tile in the level overviews.
const BLOCK: usize = 3;
/// Pixels a tile in the film of the best moment.
const FILM_CELL: usize = 12;

/// What an export wrote.
#[derive(Debug, Default)]
pub struct Exported {
    pub dir: PathBuf,
    /// The levels pictured, top first.
    pub levels: Vec<i32>,
    /// The best moment was filmed as a GIF.
    pub gif: bool,
    pub files: Vec<String>,
}

impl Exported {
    pub fn summary(&self) -> String {
        format!(
            "Exported {} levels{} and the fort's story to {}.",
            self.levels.len(),
            if self.gif { ", the best moment as a GIF," } else { "" },
            self.dir.display()
        )
    }
}

/// The levels worth a picture: any with a fair stretch of ground the fort
/// has seen, from the top down.
pub fn levels(g: &Game) -> Vec<i32> {
    let w = &g.world;
    (0..w.d)
        .rev()
        .filter(|&z| {
            let mut n = 0;
            for y in 0..w.h {
                for x in 0..w.w {
                    let t = w.t(Pos::new(x, y, z));
                    if t.revealed && t.kind.has_floor() {
                        n += 1;
                        if n >= 30 {
                            return true;
                        }
                    }
                }
            }
            false
        })
        .collect()
}

/// The folder an export goes to: `exports/<fort>-<year>`.
pub fn default_dir(g: &Game) -> PathBuf {
    let name: String = g.fortress_name.chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    Path::new("exports").join(format!("{}-{}", name.trim_matches('-'), g.start_year as u64 + g.tick / YEAR))
}

/// Write the export to `dir` (or `default_dir`).
pub fn export_fort(app: &mut App, dir: Option<&Path>) -> Result<Exported, String> {
    let dir = dir.map(Path::to_path_buf).unwrap_or_else(|| default_dir(&app.game));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    let mut out = Exported { dir: dir.clone(), ..Default::default() };
    let write = |out: &mut Exported, name: &str, bytes: &[u8]| -> Result<(), String> {
        std::fs::write(dir.join(name), bytes).map_err(|e| format!("{}: {}", name, e))?;
        out.files.push(name.to_string());
        Ok(())
    };
    let (w, h) = (app.game.world.w, app.game.world.h);
    for z in levels(&app.game) {
        let cells = capture(app, Pos::new(0, 0, z), w, h);
        let text: String = cells.chunks(w as usize).map(|row| row.iter().map(|c| c.0).collect::<String>() + "\n").collect();
        write(&mut out, &format!("level-z{}.txt", z), text.as_bytes())?;
        if let Some(png) = overview_png(&cells, w as usize, h as usize) {
            write(&mut out, &format!("level-z{}.png", z), &png)?;
        }
        out.levels.push(z);
    }
    let pick = best_moment(&app.game.reel.clips);
    if let Some(c) = pick.and_then(|i| app.game.reel.clips.get(i)) {
        write(&mut out, "best-moment.gif", &clip_gif(c))?;
        out.gif = true;
    }
    write(&mut out, "story.txt", story(&app.game).as_bytes())?;
    let html = page(&app.game, &out, pick);
    write(&mut out, "fort.html", html.as_bytes())?;
    Ok(out)
}

/// The moment the fort would show first: a star of the overseer's, else
/// the weightiest of the best (the latest of equals).
fn best_moment(clips: &[Clip]) -> Option<usize> {
    best(clips).into_iter().max_by_key(|&i| (clips[i].star, clips[i].score, clips[i].tick))
}

/// One level as the world map draws the world: a small block a tile, the
/// tile's paper with its ink in a cross over it (solid where the tile is).
pub fn overview_pixels(cells: &[Look], w: usize, h: usize) -> Vec<u32> {
    let pw = w * BLOCK;
    let mut px = vec![0u32; pw * h * BLOCK];
    let pack = |c: (u8, u8, u8)| (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32;
    for (i, &(ch, ink, paper)) in cells.iter().enumerate().take(w * h) {
        let (x0, y0) = (i % w * BLOCK, i / w * BLOCK);
        let full = matches!(ch, '█' | '▓' | '#' | '■' | '♣' | '♠');
        for dy in 0..BLOCK {
            for dx in 0..BLOCK {
                let mid = dx == BLOCK / 2 || dy == BLOCK / 2;
                let c = if ch == ' ' { paper } else if full || mid { ink } else { paper };
                px[(y0 + dy) * pw + x0 + dx] = pack(c);
            }
        }
    }
    px
}

#[cfg(feature = "window")]
fn overview_png(cells: &[Look], w: usize, h: usize) -> Option<Vec<u8>> {
    Some(crate::window::picture::png(&overview_pixels(cells, w, h), w * BLOCK, h * BLOCK))
}

#[cfg(not(feature = "window"))]
fn overview_png(_: &[Look], _: usize, _: usize) -> Option<Vec<u8>> {
    None
}

/// A frame of a clip as pixels, the way the window draws it.
#[cfg(feature = "window")]
fn frame_pixels(c: &Clip, i: usize) -> (Vec<u32>, usize, usize) {
    crate::window::picture::paint(&c.frame_cells(i), CLIP_W as usize, CLIP_H as usize, FILM_CELL)
}

/// Without the window: blocks, as in the overviews but bigger.
#[cfg(not(feature = "window"))]
fn frame_pixels(c: &Clip, i: usize) -> (Vec<u32>, usize, usize) {
    let _ = FILM_CELL;
    let cells = c.frame_cells(i);
    (overview_pixels(&cells, CLIP_W as usize, CLIP_H as usize), CLIP_W as usize * BLOCK, CLIP_H as usize * BLOCK)
}

/// A clip as a looping GIF, eight frames a second, holding on the last.
pub fn clip_gif(c: &Clip) -> Vec<u8> {
    let frames: Vec<(Vec<u32>, usize, usize)> = (0..c.frames.len().max(1)).map(|i| frame_pixels(c, i)).collect();
    let (w, h) = (frames[0].1, frames[0].2);
    let n = frames.len();
    gif::encode(w, h, frames.into_iter().enumerate().map(|(i, f)| (f.0, if i + 1 == n { 150 } else { 12 })))
}

/// A GIF encoder small enough to keep here: the 6x6x6 colour cube for a
/// palette (the reel's colours are already on it), and LZW codes written
/// out one pixel each, the table cleared before it can grow, which every
/// reader takes and needs no compressor.
pub mod gif {
    fn level(v: u8) -> u8 {
        ((v as u32 * 5 + 127) / 255) as u8
    }

    /// The palette index of a pixel.
    pub fn index(p: u32) -> u8 {
        let (r, g, b) = ((p >> 16) as u8, (p >> 8) as u8, p as u8);
        level(r) * 36 + level(g) * 6 + level(b)
    }

    struct Bits {
        out: Vec<u8>,
        acc: u32,
        n: u32,
    }

    impl Bits {
        fn put(&mut self, code: u32, width: u32) {
            self.acc |= code << self.n;
            self.n += width;
            while self.n >= 8 {
                self.out.push(self.acc as u8);
                self.acc >>= 8;
                self.n -= 8;
            }
        }
    }

    /// `frames` of `w` x `h` pixels (0x00RRGGBB), each with its delay in
    /// hundredths of a second. After the first, a frame is only the box
    /// around what changed, drawn over the one before.
    pub fn encode(w: usize, h: usize, frames: impl Iterator<Item = (Vec<u32>, u16)>) -> Vec<u8> {
        let mut o = b"GIF89a".to_vec();
        o.extend_from_slice(&(w as u16).to_le_bytes());
        o.extend_from_slice(&(h as u16).to_le_bytes());
        o.extend_from_slice(&[0xF7, 0, 0]);
        for i in 0..256u32 {
            let i = i.min(215);
            o.extend_from_slice(&[(i / 36 * 51) as u8, (i / 6 % 6 * 51) as u8, (i % 6 * 51) as u8]);
        }
        // Loop for ever.
        o.extend_from_slice(&[0x21, 0xFF, 0x0B]);
        o.extend_from_slice(b"NETSCAPE2.0");
        o.extend_from_slice(&[3, 1, 0, 0, 0]);
        let mut prev: Option<Vec<u32>> = None;
        for (px, delay) in frames {
            // The box of what changed (all of it, the first time).
            let (mut x0, mut y0, mut x1, mut y1) = (0, 0, w, h);
            if let Some(p) = &prev {
                (x0, y0, x1, y1) = (w, h, 0, 0);
                for y in 0..h {
                    for x in 0..w {
                        if p[y * w + x] != px[y * w + x] {
                            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                        }
                    }
                }
                if x1 == 0 {
                    (x0, y0, x1, y1) = (0, 0, 1, 1);
                }
            }
            o.extend_from_slice(&[0x21, 0xF9, 4, 4]);
            o.extend_from_slice(&delay.to_le_bytes());
            o.extend_from_slice(&[0, 0, 0x2C]);
            for v in [x0, y0, x1 - x0, y1 - y0] {
                o.extend_from_slice(&(v as u16).to_le_bytes());
            }
            o.extend_from_slice(&[0, 8]);
            let mut bits = Bits { out: Vec::new(), acc: 0, n: 0 };
            let inside = (y0..y1).flat_map(|y| (x0..x1).map(move |x| y * w + x));
            for (k, &p) in inside.map(|i| &px[i]).enumerate() {
                // A clear before the table would need a wider code.
                if k % 250 == 0 {
                    bits.put(256, 9);
                }
                bits.put(index(p) as u32, 9);
            }
            bits.put(257, 9);
            if bits.n > 0 {
                bits.out.push(bits.acc as u8);
            }
            for chunk in bits.out.chunks(255) {
                o.push(chunk.len() as u8);
                o.extend_from_slice(chunk);
            }
            o.push(0);
            prev = Some(px);
        }
        o.push(0x3B);
        o
    }

    /// Back from the codes to the pixels' indices (for the tests).
    #[cfg(test)]
    pub fn decode_codes(data: &[u8], count: usize) -> Vec<u8> {
        let (mut acc, mut n, mut out, mut it) = (0u32, 0u32, Vec::new(), data.iter());
        while out.len() < count {
            while n < 9 {
                acc |= (*it.next().unwrap() as u32) << n;
                n += 8;
            }
            let code = acc & 0x1FF;
            acc >>= 9;
            n -= 9;
            if code < 256 {
                out.push(code as u8);
            }
        }
        out
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The fort's story, section by section: (heading, lines).
fn sections(g: &Game) -> Vec<(String, Vec<String>)> {
    let mut v = Vec::new();
    for c in &g.chronicles {
        let mut lines = c.lines.clone();
        let works = g.works_of_year(c.year);
        if !works.is_empty() {
            lines.push(format!("Works of the year: {}.", works.join("; ")));
        }
        v.push((format!("Year {}: {}", c.year, c.title), lines));
    }
    let sagas: Vec<String> = g
        .told_sagas()
        .iter()
        .map(|s| {
            let mut t = format!("{}. ", s.title);
            for c in &s.chapters {
                t.push_str(&c.text);
                t.push(' ');
            }
            t.push_str(&s.closing);
            t.trim().to_string()
        })
        .collect();
    if !sagas.is_empty() {
        v.push(("The sagas".to_string(), sagas));
    }
    let battles: Vec<String> = g
        .annals
        .sieges
        .iter()
        .map(|s| {
            let name = s.name.clone().unwrap_or_else(|| format!("The siege of {}", g.short_date(s.tick).trim()));
            let how = if s.outcome.is_empty() { "unfinished".to_string() } else { s.outcome.clone() };
            let led = if s.leader.is_empty() { String::new() } else { format!(", led by {}", s.leader) };
            format!("{} ({}): {} {}{}, {}. {} slain, {} dwarves lost.", name, g.short_date(s.tick).trim(), s.size, s.foe, led, how, s.foes_slain, s.dwarves_lost)
        })
        .collect();
    if !battles.is_empty() {
        v.push(("Battles".to_string(), battles));
    }
    let fallen: Vec<String> = g
        .annals
        .deaths
        .iter()
        .map(|d| {
            let by = d.killer.as_ref().map_or(String::new(), |k| format!(" ({})", k));
            let words = d.last_words.as_ref().map_or(String::new(), |w| format!(" Their last words: \"{}\"", w));
            format!("{}: {} {}{}.{}", g.short_date(d.tick).trim(), d.name, d.cause, by, words)
        })
        .collect();
    if !fallen.is_empty() {
        v.push(("The fallen".to_string(), fallen));
    }
    let reel: Vec<String> = g.reel.clips.iter().map(|c| format!("{}{} ({}): {}", if c.star { "★ " } else { "" }, c.title, g.short_date(c.tick).trim(), c.caption)).collect();
    if !reel.is_empty() {
        v.push(("The highlight reel".to_string(), reel));
    }
    if let Some(o) = &g.overseer {
        let pages: Vec<String> = o.journal.iter().map(|p| format!("{} ({}): {}", g.short_date(p.tick).trim(), p.by, p.lines.join(" "))).collect();
        if !pages.is_empty() {
            v.push(("The overseer's journal".to_string(), pages));
        }
    }
    v
}

fn heading(g: &Game) -> (String, String) {
    let alive = g.alive_count();
    let state = if g.fallen || (alive == 0 && !g.dwarves.is_empty()) { "fallen".to_string() } else { format!("{} dwarves alive", alive) };
    (g.fortress_name.clone(), format!("{} · {}", g.date_string(), state))
}

/// The story as plain text.
pub fn story(g: &Game) -> String {
    let (name, sub) = heading(g);
    let mut s = format!("{}\n{}\n{}\n", name, "=".repeat(name.chars().count()), sub);
    for (h, lines) in sections(g) {
        s.push_str(&format!("\n{}\n{}\n", h, "-".repeat(h.chars().count())));
        for l in lines {
            s.push_str(&l);
            s.push('\n');
        }
    }
    s
}

/// The story as a page, with the level pictures and the best moment.
fn page(g: &Game, out: &Exported, pick: Option<usize>) -> String {
    let (name, sub) = heading(g);
    let mut s = String::new();
    s.push_str("<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>");
    s.push_str(&esc(&name));
    s.push_str("</title>\n<style>\nbody{background:#14100c;color:#e8d6aa;font-family:Georgia,serif;max-width:60em;margin:2em auto;padding:0 1em;line-height:1.5}\nh1{color:#ffd73c;margin-bottom:0}h2{color:#d9a441;border-bottom:1px solid #5a4528}\n.sub{color:#a08060}img{image-rendering:pixelated;max-width:100%;border:1px solid #5a4528}\nfigure{margin:1em 0}figcaption{color:#a08060;font-size:.9em}.star{color:#ffd73c}\nnav a{color:#d9a441;margin-right:.6em}\n</style></head><body>\n");
    s.push_str(&format!("<h1>{}</h1>\n<p class=\"sub\">{}</p>\n", esc(&name), esc(&sub)));
    if let Some(c) = pick.and_then(|i| g.reel.clips.get(i)).filter(|_| out.gif) {
        s.push_str(&format!(
            "<h2>The best moment</h2>\n<figure><img src=\"best-moment.gif\" alt=\"{}\"><figcaption><b>{}</b> ({}). {}</figcaption></figure>\n",
            esc(&c.title),
            esc(&c.title),
            esc(g.short_date(c.tick).trim()),
            esc(&c.caption)
        ));
    }
    if !out.levels.is_empty() {
        s.push_str("<h2>The levels</h2>\n<nav>");
        for z in &out.levels {
            s.push_str(&format!("<a href=\"#z{}\">z {}</a>", z, z));
        }
        s.push_str("</nav>\n");
        for z in &out.levels {
            let png = format!("level-z{}.png", z);
            let img = if out.files.contains(&png) { format!("<img src=\"{}\" alt=\"level {}\">", png, z) } else { String::new() };
            s.push_str(&format!("<figure id=\"z{z}\">{img}<figcaption>Level {z} · <a href=\"level-z{z}.txt\">as text</a></figcaption></figure>\n", z = z, img = img));
        }
    }
    for (h, lines) in sections(g) {
        s.push_str(&format!("<h2>{}</h2>\n", esc(&h)));
        for l in lines {
            let l = esc(&l);
            let l = match l.strip_prefix("★ ") {
                Some(rest) => format!("<span class=\"star\">★</span> {}", rest),
                None => l,
            };
            s.push_str(&format!("<p>{}</p>\n", l));
        }
    }
    s.push_str("</body></html>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Mode;

    #[test]
    fn a_gif_holds_its_pixels() {
        let px: Vec<u32> = (0..600u32).map(|i| if i % 7 == 0 { 0xff0000 } else { 0x336699 }).collect();
        let g = gif::encode(30, 20, vec![(px.clone(), 12)].into_iter());
        assert_eq!(&g[..6], b"GIF89a");
        assert_eq!(*g.last().unwrap(), 0x3B);
        // The image data: after the header, palette, loop, control and
        // descriptor blocks, then the code size; gather the sub-blocks.
        let mut i = 13 + 768 + 19 + 8 + 10;
        assert_eq!(g[i], 8);
        i += 1;
        let mut data = Vec::new();
        while g[i] != 0 {
            let n = g[i] as usize;
            data.extend_from_slice(&g[i + 1..i + 1 + n]);
            i += n + 1;
        }
        let back = gif::decode_codes(&data, 600);
        assert_eq!(back, px.iter().map(|&p| gif::index(p)).collect::<Vec<_>>());
        assert_eq!(gif::index(0xff0000), 180);
    }

    /// The Done-when of UI-10: a fort exports its levels and its story.
    #[test]
    fn a_fort_exports_its_levels_and_story() {
        let mut g = crate::game::trial_game(5003, crate::game::Style::Steady, 30);
        let at = g.dwarves[0].pos;
        g.film_for_test("An artifact", "Urist has created a thing of beauty!", Some(at));
        let mut app = App::with_game_for_tests(g);
        app.mode = Mode::Highlights { sel: 0 };
        let dir = std::env::temp_dir().join(format!("dwarfclone-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let e = export_fort(&mut app, Some(&dir)).expect("exported");
        assert!(!e.levels.is_empty());
        for z in &e.levels {
            let txt = std::fs::read_to_string(dir.join(format!("level-z{}.txt", z))).unwrap();
            assert_eq!(txt.lines().count(), app.game.world.h as usize);
            #[cfg(feature = "window")]
            assert!(std::fs::read(dir.join(format!("level-z{}.png", z))).unwrap().starts_with(b"\x89PNG"));
        }
        // The surface the fort stands on is pictured, and every dug level.
        let wagon = app.game.embark.z;
        assert!(e.levels.contains(&wagon), "{:?} lacks {}", e.levels, wagon);
        let html = std::fs::read_to_string(dir.join("fort.html")).unwrap();
        assert!(html.contains(&esc(&app.game.fortress_name)) && html.contains("best-moment.gif") && html.contains("thing of beauty"));
        let story = std::fs::read_to_string(dir.join("story.txt")).unwrap();
        assert!(story.starts_with(&app.game.fortress_name) && story.contains("The highlight reel"));
        assert!(std::fs::read(dir.join("best-moment.gif")).unwrap().starts_with(b"GIF89a"));
        // The screen came back as it was.
        assert_eq!(app.mode, Mode::Highlights { sel: 0 });
        let _ = std::fs::remove_dir_all(&dir);
    }
}
