//! Pieces shared by the underground scenes (set B): rock seen in section,
//! hollows, torches and their flicker, dwarves' beards, and the beat of an
//! event that repeats through a launch but starts somewhere new each time.

use super::*;

/// Frame `0..period` of an event that keeps repeating, started at a
/// different point each launch (`k` picks which of the seed's numbers).
pub(super) fn beat(cv: &Canvas, period: u64, k: u32) -> u64 {
    let p = period.max(1);
    (cv.t + cv.roll(k) as u64 % p) % p
}

/// How bright flame `k` burns at frame `t`, about 0.5..1.
pub(super) fn flicker(t: u64, k: i32) -> f32 {
    let tf = t as f32;
    (((tf * 0.31 + k as f32 * 1.7).sin() * 0.5 + (tf * 0.83 + k as f32 * 0.6).sin() * 0.3) / 1.6 + 0.75).clamp(0.3, 1.0)
}

/// Solid rock over rows `y0..y1`: wavy strata in shades of `base`, a few
/// ore flecks, darkening with depth.
pub(super) fn bedrock(cv: &mut Canvas, y0: i32, y1: i32, base: Rgb, salt: i32) {
    let (w, h) = (cv.w(), cv.h().max(1));
    let tones = [base, lerp(base, (255, 240, 220), 0.1), lerp(base, (0, 0, 0), 0.18), lerp(base, (base.2, base.0, base.1), 0.18), lerp(base, (255, 240, 220), 0.04)];
    let ph = (salt % 628) as f32 / 100.0;
    let waves: Vec<f32> = (0..w).map(|x| (x as f32 * 0.045 + ph).sin() * 1.8 + (x as f32 * 0.13 - ph).sin() * 0.6).collect();
    let ore = [(200, 120, 70), (235, 195, 80), (205, 205, 220), (120, 200, 140)][(salt as u32 % 4) as usize];
    for y in y0.max(0)..y1.min(cv.h()) {
        let deep = y as f32 / h as f32 * 0.45;
        for x in 0..w {
            let band = ((y as f32 + waves[x as usize]) / 3.3).floor() as i32;
            let tone = lerp(tones[(hash(band, salt) % 5) as usize], (0, 0, 0), deep);
            let hs = hash(x * 3 + salt, y * 7 + 1);
            let (ch, fg) = if hash(x / 3 + salt, (y + band) / 2) % 157 == 0 && hs % 3 != 0 {
                ('•', lerp(ore, tone, 0.5))
            } else {
                match hs % 43 {
                    0..=3 => ('░', lerp(tone, (0, 0, 0), 0.35)),
                    4 => ('▒', lerp(tone, (150, 150, 150), 0.12)),
                    5 => ('·', lerp(tone, (210, 200, 180), 0.35)),
                    _ => (' ', tone),
                }
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(tone)));
        }
    }
}

/// The first row clear of the title menu, where the lower half of a
/// scene can go.
pub(super) fn below(cv: &Canvas) -> i32 {
    cv.sky + 14
}

/// Open air from (`x0`, `y0`) up to (`x1`, `y1`), not included.
pub(super) fn hollow(cv: &mut Canvas, x0: i32, y0: i32, x1: i32, y1: i32, air: Rgb) {
    for y in y0.max(0)..y1.min(cv.h()) {
        for x in x0.max(0)..x1.min(cv.w()) {
            cv.fill(x, y, rgb(air));
        }
    }
}

/// A worked floor along row `y`, from `x0` up to `x1`.
pub(super) fn floor(cv: &mut Canvas, x0: i32, x1: i32, y: i32, col: Rgb) {
    for x in x0.max(0)..x1.min(cv.w()) {
        cv.put(x, y, '▀', rgb(col), Some(rgb(lerp(col, (0, 0, 0), 0.6))));
    }
}

/// A torch on the wall at (`x`, `y`), its light reaching about `reach`.
pub(super) fn torch(cv: &mut Canvas, x: i32, y: i32, k: i32, reach: f32) {
    let f = flicker(cv.t, k);
    cv.halo(x, y, reach * (0.85 + 0.15 * f), (255, 160, 70), 0.55 * f);
    cv.glyph(x, y, '¡', rgb(lerp((200, 120, 50), (255, 230, 140), f)));
}

/// A beard colour for dwarf `k`.
pub(super) fn beard(k: u32) -> Rgb {
    [(235, 190, 110), (210, 140, 90), (200, 170, 220), (150, 200, 240), (240, 160, 120), (190, 220, 150)][(k % 6) as usize]
}

/// 0..1 through `a..b` of a beat, clamped.
pub(super) fn span(at: u64, a: u64, b: u64) -> f32 {
    if at <= a {
        0.0
    } else if at >= b {
        1.0
    } else {
        (at - a) as f32 / (b - a).max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set B's scenes, each with the glyph its turn of events shows and
    /// whether that comes and goes.
    const OURS: [(&str, char, bool); 16] = [
        ("cavein", '●', true),
        ("drowned", '°', false),
        ("clockwork", '◘', false),
        ("smelter", '▬', false),
        ("engraver", '☼', true),
        ("traphall", 'g', true),
        ("pitbridge", 'g', true),
        ("beast", '◉', true),
        ("spider", '@', false),
        ("crypt", 'V', true),
        ("ghosts", '☺', false),
        ("tantrum", '#', true),
        ("mood", '☼', true),
        ("mushroom", '♠', true),
        ("ambush", 'g', true),
        ("thief", 'k', true),
    ];

    fn keep(w: u16) -> Rect {
        let bw = 54u16;
        Rect::new((w.saturating_sub(bw) / 2).saturating_sub(1), 0, bw + 2, 6).intersection(Rect::new(0, 0, w, 60))
    }

    fn idx(name: &str) -> usize {
        SCENES.iter().position(|s| s.name == name).unwrap_or_else(|| panic!("{} is missing", name))
    }

    fn render(scene: usize, seed: u64, w: u16, h: u16, t: u64) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut b = Buffer::empty(area);
        draw_pick(&mut b, area, t, keep(w), Pick { scene, seed });
        b
    }

    fn text(b: &Buffer) -> String {
        b.content.iter().map(|c| c.symbol()).collect()
    }

    /// Each scene's turn of events shows up over a couple of minutes.
    #[test]
    fn set_b_scenes_tell_their_stories() {
        for (name, glyph, comes_and_goes) in OURS {
            for seed in [4u64, 8, 15] {
                let seen: Vec<bool> = (0..3000u64).step_by(20).map(|t| text(&render(idx(name), seed, 120, 40, t)).contains(glyph)).collect();
                assert!(seen.iter().any(|&s| s), "{} never shows {:?} (seed {})", name, glyph, seed);
                if comes_and_goes {
                    assert!(seen.iter().any(|&s| !s), "{} always shows {:?} (seed {})", name, glyph, seed);
                }
            }
        }
    }

    /// Square tiles as the window draws them make for tall screens: the
    /// scenes still fill them and still move.
    #[test]
    fn set_b_scenes_fill_tall_square_screens() {
        for (name, _, _) in OURS {
            let a = render(idx(name), 3, 120, 67, 300);
            let b = render(idx(name), 3, 120, 67, 360);
            assert_ne!(a, b, "{} stands still", name);
            let blank = a.content.iter().filter(|c| c.symbol() == " " && c.bg == Color::Reset).count();
            assert!(blank < 120 * 67 / 50, "{} leaves {} cells bare", name, blank);
        }
    }

    /// A picture of each of set B's scenes, as the terminal (rough glyphs)
    /// and as the window's square tiles, in target/splashb/:
    /// `cargo test --release picture_set_b -- --ignored` (SCENE=name, FRAME=n, SEED=n).
    #[test]
    #[ignore]
    fn picture_set_b() {
        let t: u64 = std::env::var("FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(333);
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(11);
        let only = std::env::var("SCENE").ok();
        let dir = std::path::PathBuf::from("target/splashb");
        let _ = std::fs::create_dir_all(&dir);
        for (name, _, _) in OURS {
            if only.as_deref().map_or(false, |o| !o.split(',').any(|n| n == name)) {
                continue;
            }
            for (tag, w, h) in [("term", 120u16, 40u16), ("window", 120, 67)] {
                let mut b = render(idx(name), seed, w, h, t);
                let gold = Color::Rgb(250, 200, 80);
                let title = "D W A R F    C L O N E";
                let x0 = w.saturating_sub(title.len() as u16) / 2;
                for row in 1..5u16 {
                    b.set_string(x0, row, title, ratatui::style::Style::default().fg(gold));
                }
                let sky = sky_height(h);
                let (mw, mh) = (66u16.min(w), 12u16);
                let r = Rect::new((w - mw) / 2, sky + 1, mw, mh).intersection(b.area);
                for y in r.y..r.y + r.height {
                    for x in r.x..r.x + r.width {
                        b.get_mut(x, y).set_char(' ').set_bg(Color::Rgb(12, 12, 20));
                    }
                }
                let path = dir.join(format!("{}-{}-s{}-t{}.png", name, tag, seed, t));
                if tag == "term" {
                    snapshot::save(&b, &path);
                } else {
                    #[cfg(feature = "window")]
                    {
                        let c = |c: Color| match c {
                            Color::Rgb(r, g, b) => (r, g, b),
                            _ => (0, 0, 0),
                        };
                        let cells: Vec<crate::window::picture::Look> = b.content.iter().map(|cell| (cell.symbol().chars().next().unwrap_or(' '), c(cell.fg), c(cell.bg))).collect();
                        let (px, pw, ph) = crate::window::picture::paint(&cells, w as usize, h as usize, 16);
                        std::fs::write(&path, crate::window::picture::png(&px, pw, ph)).unwrap();
                    }
                }
            }
        }
    }
}
