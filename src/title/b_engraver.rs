//! The engraver: a dwarf on a hanging plank works across a great smoothed
//! wall, and the picture comes out of the stone line by line, a sun, the
//! mountains, and a deed worth remembering. Finished, it catches the
//! light and the others come to look; then the wall is smoothed for the
//! next. The seed picks the stone, the deed and how it is laid out.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "engravings remember what the world forgets",
    "a masterful image of a dwarf and a very large axe",
    "carve it deep. history is a long time",
    "the stone was waiting for this picture",
];

/// Stone for the wall: face, carving shadow, carving light.
const STONES: [(Rgb, Rgb, Rgb); 4] = [
    ((180, 176, 168), (70, 66, 62), (250, 248, 240)),  // marble
    ((38, 34, 44), (10, 8, 14), (150, 140, 190)),      // obsidian
    ((70, 120, 96), (24, 50, 40), (170, 230, 200)),    // jade
    ((150, 100, 70), (70, 40, 26), (240, 190, 140)),   // red sandstone
];

/// The figures a deed is told with.
const DWARF: [&str; 3] = [" ☺/", "/█ ", " ╨ "];
const BEAST: [&str; 4] = ["  ▄▀▀▄  ", "▄██████▀", "█▀ ▀ ▀█ ", "╨  ╨ ╨  "];
const GATE: [&str; 3] = ["▄█▄█▄█▄", "█ ▄▄▄ █", "█ █ █ █"];
const TREE: [&str; 2] = ["♣", "│"];
const CROWN: [&str; 2] = ["♦▲♦", "▀▀▀"];
const SHIP: [&str; 3] = ["  │  ", " ▐█▌ ", "▀▀▀▀▀"];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let salt = cv.salt();
    let (face, shade, light) = STONES[(cv.roll(1) % 4) as usize];
    let deed = cv.roll(2) % 4;
    let hall = lerp(face, (10, 8, 8), 0.82);

    // The hall around the wall.
    hollow(cv, 0, 0, w, g, hall);
    floor(cv, 0, w, g, lerp(face, (60, 50, 40), 0.5));

    // The panel.
    let (px0, px1) = (2, w - 2);
    let (py0, py1) = (1, (g - 2).max(2));
    let (pw, ph) = ((px1 - px0).max(1), (py1 - py0).max(1));
    // The picture, laid out in the panel's own cells.
    let mut img = vec![' '; (pw * ph) as usize];
    {
        let mut set = |x: i32, y: i32, ch: char| {
            if x >= 0 && y >= 0 && x < pw && y < ph && ch != ' ' {
                img[(y * pw + x) as usize] = ch;
            }
        };
        // The sun, off to one side so the banner never hides it.
        let sun_x = if cv.roll(3) % 2 == 0 { pw / 8 + 1 } else { pw - pw / 8 - 2 };
        let sun_y = (ph / 5).min(ph - 1);
        set(sun_x, sun_y, '☼');
        if pw > 30 {
            for (dx, dy, ch) in [(-2, 0, '─'), (2, 0, '─'), (0, -1, '│'), (0, 1, '│')] {
                set(sun_x + dx, sun_y + dy, ch);
            }
        }
        // Mountains along the back.
        let horizon = ph * 2 / 3;
        for x in 0..pw {
            let m = peaks(x, pw, cv.seed, 50, 4);
            let top = horizon - (m * (ph as f32 * 0.45)) as i32;
            let next = horizon - (peaks(x + 1, pw, cv.seed, 50, 4) * (ph as f32 * 0.45)) as i32;
            let ch = if next < top { '/' } else if next > top { '\\' } else { '_' };
            set(x, top, ch);
            // Hatched flanks, and the plain below them dotted.
            for y in top + 1..horizon {
                if (x + y) % 3 == 0 {
                    set(x, y, '░');
                }
            }
            for y in horizon..ph - 1 {
                if (x * 3 + y * 5) % 7 == 0 {
                    set(x, y, '·');
                }
            }
        }
        // Clouds and birds.
        for k in 0..(pw / 14).max(1) {
            let cx = (hash(k, 31) % pw.max(1) as u32) as i32;
            let cy = (hash(k, 37) % (ph as u32 / 3).max(1)) as i32;
            for dx in 0..4 + (k % 3) {
                set(cx + dx, cy, if dx % 2 == 0 { '~' } else { '-' });
            }
            set(cx + 7, cy + 1, 'v');
        }
        set(sun_x, sun_y, '☼');
        // The ground line.
        for x in 0..pw {
            set(x, ph - 1, if x % 2 == 0 { '═' } else { '─' });
        }
        // The deed, told along the ground.
        let base = ph - 1;
        let place = |set: &mut dyn FnMut(i32, i32, char), x: i32, sprite: &[&str]| {
            let y0 = base - sprite.len() as i32;
            for (dy, r) in sprite.iter().enumerate() {
                for (dx, ch) in r.chars().enumerate() {
                    set(x + dx as i32, y0 + dy as i32, ch);
                }
            }
        };
        let n = (pw / 12).max(1);
        match deed {
            // A dwarf (or several) against a beast.
            0 => {
                place(&mut set, pw * 3 / 5, &BEAST);
                for k in 0..n.min(3) {
                    place(&mut set, pw * 3 / 5 - 5 - k * 5, &DWARF);
                }
            }
            // The founding: dwarves before a gate, a tree beside.
            1 => {
                place(&mut set, pw / 2 - 3, &GATE);
                for k in 0..n.min(4) {
                    place(&mut set, pw / 2 - 7 - k * 4, &DWARF);
                }
                place(&mut set, pw / 2 + 6, &TREE);
                place(&mut set, pw / 2 + 9, &TREE);
            }
            // A crowning.
            2 => {
                place(&mut set, pw / 2 - 1, &CROWN);
                place(&mut set, pw / 2 - 1, &DWARF);
                for k in 0..n.min(4) {
                    place(&mut set, pw / 2 - 6 - k * 4, &DWARF);
                    place(&mut set, pw / 2 + 4 + k * 4, &DWARF);
                }
            }
            // A ship come in over the water.
            _ => {
                for x in 0..pw {
                    set(x, base - 1, if (x / 2) % 2 == 0 { '~' } else { ' ' });
                }
                place(&mut set, pw / 2 + 2, &SHIP);
                for k in 0..n.min(3) {
                    place(&mut set, pw / 6 + k * 4, &DWARF);
                }
            }
        }
    }

    // The beat: carving row by row, admiring, smoothing for the next.
    let carve = 1500u64;
    let admire = 420u64;
    let period = carve + admire + 120;
    let at = beat(cv, period, 4);
    let per_row = (carve / ph as u64).max(1);
    let done = (at as f32 / carve as f32).min(1.0);
    let (row, col) = if at < carve { ((at / per_row) as i32, ((at % per_row) as f32 / per_row as f32 * pw as f32) as i32) } else { (ph, 0) };
    let smoothing = if at >= carve + admire { span(at, carve + admire, period) } else { 0.0 };
    let gleam = if at >= carve && at < carve + admire { Some((at - carve) as i32) } else { None };

    for y in 0..ph {
        for x in 0..pw {
            let (sx, sy) = (px0 + x, py0 + y);
            let polish = if hash(sx + salt, sy) % 37 == 0 && (t / 8 + hash(sx, sy) as u64) % 19 == 0 { 0.25 } else { 0.0 };
            let grain = (hash(sx / 2 + salt, sy) % 5) as f32 * 0.02;
            let mut bg = lerp(lerp(face, (0, 0, 0), y as f32 / ph as f32 * 0.25 + grain), (255, 255, 255), polish);
            let carved = (y < row || (y == row && x < col)) && (y as f32) >= smoothing * ph as f32;
            let ch = img[(y * pw + x) as usize];
            if let Some(gl) = gleam {
                let d = (x + y * 2 - gl / 2).abs();
                if d < 4 {
                    bg = lerp(bg, light, 0.35 * (1.0 - d as f32 / 4.0));
                }
            }
            if carved && ch != ' ' {
                let fg = if ch == '☼' || ch == '♦' { light } else { shade };
                cv.put(sx, sy, ch, rgb(fg), Some(rgb(lerp(bg, shade, 0.12))));
            } else {
                cv.put(sx, sy, ' ', Color::Black, Some(rgb(bg)));
            }
        }
    }
    // The frame.
    for x in px0 - 1..=px1 {
        cv.glyph(x, py0 - 1, '▄', rgb(lerp(face, (0, 0, 0), 0.4)));
        cv.glyph(x, py1, '▀', rgb(lerp(face, (0, 0, 0), 0.4)));
    }
    for y in py0..py1 {
        cv.put(px0 - 1, y, '▐', rgb(lerp(face, (0, 0, 0), 0.4)), None);
        cv.put(px1, y, '▌', rgb(lerp(face, (0, 0, 0), 0.4)), None);
    }

    // The engraver on the plank, chips falling.
    let b = beard(cv.roll(5));
    if at < carve {
        let (ex, ey) = (px0 + col, py0 + row);
        let tap = (t / 4) % 2 == 0;
        cv.glyph(ex + 1, ey, '☺', rgb(b));
        cv.glyph(ex, ey, if tap { '/' } else { '─' }, rgb((200, 200, 210)));
        for x in ex - 1..=ex + 3 {
            cv.glyph(x, ey + 1, '▬', rgb((130, 95, 60)));
        }
        // The ropes it hangs from.
        for y in 0..=ey {
            cv.glyph(ex - 1, y, '│', rgb((150, 120, 80)));
            cv.glyph(ex + 3, y, '│', rgb((150, 120, 80)));
        }
        for k in 0..3 {
            let age = ((t / 2) as i32 + k * 4) % 10;
            let y = ey + 2 + age;
            if y < g {
                cv.glyph(ex + (k % 2), y, '·', rgb(light));
            }
        }
    } else if at < carve + admire {
        // Admirers.
        let n = 2 + (cv.roll(6) % 3) as i32;
        for k in 0..n {
            let x = w / 4 + k * (w / 2 / n.max(1)) + ((t / 30 + k as u64) % 3) as i32;
            cv.glyph(x, g - 1, '☺', rgb(beard(cv.roll(7) + k as u32)));
            if (t / 20 + k as u64) % 4 == 0 {
                cv.glyph(x, g - 2, if k % 2 == 0 { '!' } else { '♥' }, rgb((255, 220, 160)));
            }
        }
        cv.glyph(px0 + 1, g - 1, '☺', rgb(b));
    } else {
        // Smoothing the wall again for the next.
        let y = py0 + (smoothing * ph as f32) as i32;
        let x = pingpong(t, 1, px0, px1 - 1);
        cv.glyph(x, y.min(py1 - 1), '☺', rgb(b));
        cv.glyph(x + 1, y.min(py1 - 1), '▬', rgb((200, 200, 210)));
    }
    let _ = done;
    torch(cv, 1, g - 2, 1, 3.0);
    torch(cv, w - 2, g - 2, 2, 3.0);

    // Below, a gallery of older work and someone strolling past it.
    bedrock(cv, g + 1, h, lerp(face, (60, 56, 50), 0.6), salt);
    let ly = (below(cv) + h) / 2 - 2;
    if h - below(cv) >= 6 {
        hollow(cv, 1, ly, w - 1, ly + 4, hall);
        floor(cv, 1, w - 1, ly + 4, lerp(face, (60, 50, 40), 0.5));
        let minis: [&[&str]; 4] = [&["*  ▲"], &["☺/▄▀"], &["♣ ♣ "], &["~▐▌~"]];
        let mut x = 3;
        let mut k = cv.roll(8) as usize;
        while x + 6 < w {
            cv.put(x - 1, ly + 1, '[', rgb(shade), Some(rgb(face)));
            cv.sprite(x, ly + 1, minis[k % 4], rgb(shade));
            for dx in 0..4 {
                cv.tint(x + dx, ly + 1, rgb(face));
            }
            cv.put(x + 4, ly + 1, ']', rgb(shade), Some(rgb(face)));
            x += 11;
            k += 1;
        }
        let wx = pingpong(t, 6, 2, w - 3);
        cv.glyph(wx, ly + 3, '☺', rgb(beard(cv.roll(9))));
        torch(cv, w / 3, ly, 5, 3.0);
        torch(cv, w * 2 / 3, ly, 6, 3.0);
    }
}
