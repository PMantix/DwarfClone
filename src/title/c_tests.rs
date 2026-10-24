//! Tests and pictures for the world, sea and sky scenes.

use super::*;

/// The scenes of this set, and a glyph each shows while its story plays:
/// (name, any of these glyphs, comes and goes).
const SET: [(&str, &str, bool); 16] = [
    ("campfire", "▲", false),
    ("slayer", "x", false),
    ("voyage", "▪", false),
    ("lighthouse", "☺", false),
    ("worldgen", "☼■♠Ω", true),
    ("march", "↑", false),
    ("elves", "♦", false),
    ("castle", "H", false),
    ("darkfort", "g", false),
    ("fair", "▼", false),
    ("ford", "≈", true),
    ("comet", "*", false),
    ("eclipse", "¡", true),
    ("ages", "☺", false),
    ("skyisles", "║", false),
    ("volcano", "●", false),
];

fn keep(w: u16) -> Rect {
    let bw = 54u16;
    Rect::new((w.saturating_sub(bw) / 2).saturating_sub(1), 0, bw + 2, 6).intersection(Rect::new(0, 0, w, 60))
}

fn render(scene: usize, seed: u64, w: u16, h: u16, t: u64) -> Buffer {
    let area = Rect::new(0, 0, w, h);
    let mut b = Buffer::empty(area);
    draw_pick(&mut b, area, t, keep(w), Pick { scene, seed });
    b
}

fn idx(name: &str) -> usize {
    SCENES.iter().position(|s| s.name == name).unwrap_or_else(|| panic!("{} is missing", name))
}

fn shows(b: &Buffer, glyphs: &str) -> bool {
    b.content.iter().any(|c| c.symbol().chars().next().map_or(false, |g| glyphs.contains(g)))
}

#[test]
fn the_world_scenes_are_all_there_and_tell_their_stories() {
    for (name, glyphs, comes_and_goes) in SET {
        for seed in [4u64, 8, 15] {
            let seen: Vec<bool> = (0..3000u64).step_by(40).map(|t| shows(&render(idx(name), seed, 120, 40, t), glyphs)).collect();
            assert!(seen.iter().any(|&s| s), "{} never shows {:?} (seed {})", name, glyphs, seed);
            if comes_and_goes {
                assert!(seen.iter().any(|&s| !s), "{} always shows {:?} (seed {})", name, glyphs, seed);
            }
        }
    }
}

/// Pictures of this set's scenes in target/splashc/: as the terminal shows
/// them (`term-`) and as the window's square tiles do (`win-`), with the
/// banner and menu laid over roughly as the title screen lays them.
/// `cargo test --release pictures_of_the_world_scenes -- --ignored`
/// (SIZE=WxH, FRAME=n, SCENE=name,name).
#[test]
#[ignore]
fn pictures_of_the_world_scenes() {
    let (w, h) = std::env::var("SIZE")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((120u16, 40u16));
    let t: u64 = std::env::var("FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(333);
    let only = std::env::var("SCENE").ok();
    for (name, _, _) in SET {
        if only.as_deref().map_or(false, |o| !o.split(',').any(|n| n == name)) {
            continue;
        }
        for seed in [11u64, 12, 13] {
            // The terminal's tall cells at SIZE, and the window's square
            // ones at the same width (a 1920x1080 window of 16px tiles is
            // about 120x67).
            let b = dressed(idx(name), seed, w, h, t);
            super::snapshot::save(&b, &std::path::PathBuf::from(format!("target/splashc/term-{}-{}-t{}.png", name, seed, t)));
            #[cfg(feature = "window")]
            {
                let wh = (w as u32 * 9 / 16) as u16;
                let b = dressed(idx(name), seed, w, wh, t);
                let rgb_of = |c: Color, d: (u8, u8, u8)| match c {
                    Color::Rgb(r, g, b) => (r, g, b),
                    _ => d,
                };
                let cells: Vec<crate::window::picture::Look> =
                    b.content.iter().map(|c| (c.symbol().chars().next().unwrap_or(' '), rgb_of(c.fg, (200, 200, 200)), rgb_of(c.bg, (0, 0, 0)))).collect();
                let (px, pw, ph) = crate::window::picture::paint(&cells, w as usize, wh as usize, 12);
                let path = std::path::PathBuf::from(format!("target/splashc/win-{}-{}-t{}.png", name, seed, t));
                std::fs::write(&path, crate::window::picture::png(&px, pw, ph)).unwrap();
            }
        }
    }
}

/// A scene with the title and the menu laid over it roughly as the title
/// screen lays them.
fn dressed(scene: usize, seed: u64, w: u16, h: u16, t: u64) -> Buffer {
    let mut b = render(scene, seed, w, h, t);
    let gold = Color::Rgb(250, 200, 80);
    let title = "D W A R F    C L O N E";
    let x0 = w.saturating_sub(title.len() as u16) / 2;
    for row in 1..5u16.min(h) {
        b.set_string(x0, row, title, ratatui::style::Style::default().fg(gold));
    }
    let sky = sky_height(h);
    let (mw, mh) = (66u16.min(w), 12u16);
    let top = (sky + 1).min(h.saturating_sub(mh));
    let r = Rect::new((w - mw) / 2, top, mw, mh.min(h)).intersection(b.area);
    for y in r.y..r.y + r.height {
        for x in r.x..r.x + r.width {
            b.get_mut(x, y).set_char(' ').set_bg(Color::Rgb(12, 12, 20));
        }
    }
    b
}
