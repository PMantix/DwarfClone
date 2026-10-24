//! Odds and ends shared by the surface scenes (the `a_` set): clouds, rain,
//! things that drift across the screen, and pictures of the scenes as the
//! window's square tiles show them.

use super::*;

/// Where drifting thing `k` is along `0..span` at frame `t`, one step
/// every `every` frames, starting somewhere of its own.
pub(super) fn drift(t: u64, k: i32, every: u64, span: i32) -> i32 {
    ((hash(k, 7_919) as u64 + t / every.max(1)) % span.max(1) as u64) as i32
}

/// A puffy cloud `len` cells long, its left end at `x` and its flat
/// bottom on row `y + 1`, the top lit by `hi`.
pub(super) fn cloud(cv: &mut Canvas, x: i32, y: i32, len: i32, body: Rgb, hi: Rgb) {
    let len = len.max(3);
    for i in 0..len {
        let ch = if i == 0 || i == len - 1 { '▄' } else { '█' };
        cv.glyph(x + i, y + 1, ch, rgb(body));
    }
    let (a, b) = (x + len / 5, x + len - 1 - len / 4);
    for i in a..=b {
        let ch = if i == a || i == b { '▄' } else { '█' };
        cv.glyph(i, y, ch, rgb(hi));
    }
    if len >= 12 {
        let (c, d) = (x + len / 3, x + len / 2 + 1);
        for i in c..=d {
            cv.glyph(i, y - 1, '▄', rgb(hi));
        }
    }
}

/// Clouds sailing across rows `rows.0..rows.1`, `n` of them, one step
/// every `every` frames.
pub(super) fn clouds(cv: &mut Canvas, n: i32, rows: (i32, i32), every: u64, body: Rgb, hi: Rgb, salt: i32) {
    let (w, t) = (cv.w(), cv.t);
    for k in 0..n {
        let len = 6 + (hash(k + salt, 19) % (w.max(20) as u32 / 8)) as i32;
        let y = rows.0 + (hash(k + salt, 13) % (rows.1 - rows.0).max(1) as u32) as i32;
        let x = drift(t, k + salt, every + (k % 3) as u64 * every / 2, w + len * 2) - len;
        cloud(cv, x, y, len, body, hi);
    }
}

/// Rain over rows `0..rows`: `n` drops leaning `lean` cells per row
/// (`/` one way, `\` the other, `│` straight down).
pub(super) fn rain(cv: &mut Canvas, n: i32, rows: i32, lean: i32, col: Rgb, salt: i32) {
    let (w, t) = (cv.w(), cv.t);
    let rows = rows.min(cv.h()).max(1);
    let ch = match lean.signum() {
        1 => '/',
        -1 => '\\',
        _ => '│',
    };
    for k in 0..n {
        let speed = 1 + (k % 2) as u64;
        let y = ((hash(k + salt, 2) as u64 + t / speed) % rows as u64) as i32;
        let x0 = (hash(k + salt, 3) % w.max(1) as u32) as i32;
        let x = (x0 - y * lean / 2).rem_euclid(w.max(1));
        cv.glyph(x, y, ch, rgb(col));
    }
}

/// A dwarf standing at (`x`, `y`).
pub(super) fn dwarf(cv: &mut Canvas, x: i32, y: i32, coat: Rgb) {
    cv.glyph(x, y, '☺', rgb(coat));
}

/// Coats for crowds.
pub(super) const COATS: [Rgb; 8] = [(150, 200, 240), (240, 170, 110), (200, 230, 150), (230, 150, 210), (240, 220, 120), (200, 120, 100), (170, 160, 230), (120, 210, 200)];

#[cfg(all(test, feature = "window"))]
mod tests {
    use super::*;

    /// A frame of `scene` with the banner and the menu laid over it
    /// roughly as the title screen does.
    fn framed(scene: usize, seed: u64, w: u16, h: u16, t: u64) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let keep = Rect::new((w.saturating_sub(54) / 2).saturating_sub(1), 0, 56, 6).intersection(area);
        let mut b = Buffer::empty(area);
        draw_pick(&mut b, area, t, keep, Pick { scene, seed });
        let gold = Color::Rgb(250, 200, 80);
        let name = "D W A R F    C L O N E";
        let x0 = w.saturating_sub(name.len() as u16) / 2;
        for row in 1..5u16.min(h) {
            b.set_string(x0, row, name, ratatui::style::Style::default().fg(gold));
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

    /// The `a_` scenes as the window's square tiles paint them, and as the
    /// terminal sketch does, in target/splasha/:
    /// `cargo test --release picture_the_surface_scenes -- --ignored`.
    #[test]
    #[ignore]
    fn picture_the_surface_scenes() {
        let t: u64 = std::env::var("FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(333);
        let only = std::env::var("SCENE").ok();
        for scene in 0..SCENE_COUNT {
            let name = SCENES[scene].name;
            if !name.starts_with("a_") || only.as_deref().map_or(false, |o| !o.split(',').any(|n| n == name)) {
                continue;
            }
            for seed in [11u64, 12, 13] {
                // The window: square tiles, the screen about 16:9.
                let (w, h) = (96u16, 54u16);
                let b = framed(scene, seed, w, h, t);
                let cells: Vec<crate::window::picture::Look> = b
                    .content
                    .iter()
                    .map(|c| {
                        let col = |c: Color, d: (u8, u8, u8)| match c {
                            Color::Rgb(r, g, b) => (r, g, b),
                            _ => d,
                        };
                        (c.symbol().chars().next().unwrap_or(' '), col(c.fg, (200, 200, 200)), col(c.bg, (0, 0, 0)))
                    })
                    .collect();
                let (px, pw, ph) = crate::window::picture::paint(&cells, w as usize, h as usize, 16);
                let path = format!("target/splasha/window-{}-{}.png", name, seed);
                let _ = std::fs::create_dir_all("target/splasha");
                std::fs::write(&path, crate::window::picture::png(&px, pw, ph)).unwrap();
                // The terminal: tall cells, a wider screen.
                let b = framed(scene, seed, 140, 44, t);
                snapshot::save(&b, std::path::Path::new(&format!("target/splasha/term-{}-{}.png", name, seed)));
            }
        }
    }
}
