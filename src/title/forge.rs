//! The smithy at night: a furnace roaring under its chimney, bellows
//! heaving, a smith beating a glowing bar on the anvil while sparks spray
//! across the flagstones, fresh ingots cooling on the rack, steam off the
//! quench trough, and the cold night through the great arch. The seed
//! picks the metal, the weather outside and whether an apprentice works
//! the bellows.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the anvil rings until the stars go out",
    "iron remembers every blow",
    "a hot forge under a cold mountain",
    "every axe in the hall began as a spark",
];

/// Hot, warm and cold colours of the metal on the anvil.
const METALS: [(Rgb, Rgb, Rgb); 4] = [
    ((255, 245, 170), (235, 95, 30), (105, 105, 118)),  // iron
    ((255, 225, 150), (215, 105, 55), (178, 108, 70)),  // copper
    ((255, 250, 210), (240, 160, 60), (190, 190, 205)), // silver
    ((225, 255, 255), (70, 180, 235), (95, 160, 175)),  // adamantine
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = cv.sky.min(h - 1); // where the floor starts: low, so the work shows under the banner
    let tf = t as f32;
    let (hot, warm, cold) = METALS[(cv.roll(1) % 4) as usize];
    let helper = cv.roll(2) % 3 != 0;
    let outside = cv.roll(3) % 3; // clear, snow, rain
    let beard = [(205, 120, 60), (235, 235, 235), (130, 80, 45), (225, 185, 90)][(cv.roll(4) % 4) as usize];
    let big = w >= 110 && g >= 16;

    // The furnace against the near wall.
    let fw = (w / 8).clamp(7, 18).min((w / 3).max(3));
    let (f0, f1) = (1, fw);
    let fh = (g * 2 / 3).clamp(3, 15);
    let ftop = g - fh;
    let mouth_h = (fh / 2).clamp(1, 6);
    let mouth = ((f0 + f1) / 2, g - 1);
    let pump = helper && (t / 20) % 2 == 1;
    let flick = ((tf * 0.29).sin() * 0.5 + (tf * 0.71).sin() * 0.3) * 0.1 + if pump { 0.98 } else { 0.84 };
    let reach = (w as f32 / 3.2).clamp(10.0, 60.0);

    // The anvil, and the smith's stroke.
    let ax = f1 + 6 + (w as f32 * 0.1) as i32;
    let beat = 16u64;
    let p = t % beat;
    let heat = 1.0 - (t % 420) as f32 / 420.0 * 0.75;
    let bar = lerp(warm, hot, heat);
    let light = |x: i32, y: i32| -> f32 {
        let (dx, dy) = ((x - mouth.0) as f32 * 0.5, (y - mouth.1) as f32);
        let a = (1.0 - (dx * dx + dy * dy).sqrt() / reach).max(0.0) * flick;
        let (ex, ey) = ((x - ax - 1) as f32 * 0.5, (y - (g - 3)) as f32);
        let b = (1.0 - (ex * ex + ey * ey).sqrt() / 7.0).max(0.0) * 0.45 * heat;
        (a + b).min(1.0)
    };

    // Stone walls, lit where the fire reaches.
    for y in 0..g.min(h) {
        for x in 0..w {
            let l = light(x, y);
            let bg = lerp((24, 20, 24), (150, 70, 28), l * 0.9);
            let course = y % 3 == 2;
            let joint = (x + if (y / 3) % 2 == 0 { 0 } else { 5 }) % 10 == 0;
            let ch = if course { '─' } else if joint { '│' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(bg, (0, 0, 0), 0.4)), Some(rgb(bg)));
        }
    }

    // The great arch onto the night.
    let aw = (w / 10).clamp(3, 18);
    let ac = w - aw - 3;
    let trough_end = ax + 8 + 2 * (w as f32 * 0.06).clamp(3.0, 12.0) as i32 + 10;
    let arch = ac - aw > trough_end && g >= 5;
    let mut arch_left = w;
    if arch {
        arch_left = ac - aw;
        let ah = (aw as f32 * 0.5).min((g - 2) as f32 * 0.5).max(1.0);
        let top = |x: i32| -> i32 {
            let u = ((x - ac) as f32 / (aw as f32 + 0.5)).clamp(-1.0, 1.0);
            1 + ((1.0 - (1.0 - u * u).sqrt()) * ah).round() as i32
        };
        let seed = cv.seed;
        let peak = |x: i32| -> i32 {
            let m = peaks(x, w, seed, 70, 3);
            g - 1 - (m * (g as f32 * 0.35)) as i32
        };
        for x in ac - aw..=ac + aw {
            let ty = top(x);
            let pk = peak(x);
            for y in ty..g {
                let sky = lerp((6, 8, 24), (34, 30, 62), y as f32 / g.max(1) as f32);
                if y >= pk {
                    let snow = y == pk;
                    cv.put(x, y, if snow { '▲' } else { ' ' }, rgb((190, 196, 220)), Some(rgb((26, 26, 44))));
                } else if hash(x * 3 + 1, y * 7) % 13 == 0 && outside == 0 {
                    let tw = (t / 7 + x as u64) % 17 == 0;
                    cv.put(x, y, if tw { '+' } else { '·' }, rgb((200, 200, 230)), Some(rgb(sky)));
                } else {
                    cv.fill(x, y, rgb(sky));
                }
            }
            // The arch's stones.
            cv.put(x, ty - 1, '▄', rgb((96, 86, 82)), None);
        }
        for y in 1..g {
            cv.put(ac - aw - 1, y, '█', rgb((80, 72, 70)), None);
            cv.put(ac + aw + 1, y, '█', rgb((80, 72, 70)), None);
        }
        if outside == 0 && aw >= 5 {
            cv.disc(ac + aw / 2, 1 + (ah as i32) + 1, 0.8, (235, 230, 200));
        }
        // Weather blowing past.
        if outside > 0 {
            let n = (aw * g / 6).max(3);
            for k in 0..n {
                let span = (2 * aw + 1).max(1);
                let (dx, fall) = if outside == 1 { (12u64, 5u64) } else { (40, 1) };
                let x = ac - aw + ((hash(k, 5) as u64 + t / dx) % span as u64) as i32;
                let y = ((hash(k, 6) as u64 + t / fall) % g.max(1) as u64) as i32;
                if y >= top(x) && y < peak(x) {
                    let (ch, col) = if outside == 1 { ('*', (235, 238, 250)) } else { ('/', (110, 130, 180)) };
                    cv.glyph(x, y, ch, rgb(col));
                }
            }
        }
    }

    // The floor: big flagstones, warm near the fire.
    for y in g.max(0)..h {
        for x in 0..w {
            let l = light(x, y);
            let bg = lerp((34, 30, 32), (150, 80, 36), l * 0.75);
            let joint = (y - g) % 3 == 0 && y > g || (x + ((y - g) / 3) * 4) % 9 == 0;
            let ch = if y == g { '▀' } else if joint { '·' } else { ' ' };
            let fg = if y == g { lerp(bg, (110, 100, 96), 0.6) } else { lerp(bg, (0, 0, 0), 0.35) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // The furnace and its chimney.
    let stone = (84, 76, 74);
    let (c0, c1) = (f0 + fw / 3, f1 - fw / 3);
    for y in 0..g {
        for x in f0..=f1 {
            if y < ftop && (x < c0 || x > c1) {
                continue;
            }
            let l = light(x, y) * 0.5;
            let bg = lerp(stone, (170, 96, 52), l);
            let ch = if (x + (y % 2) * 2) % 4 == 0 { '│' } else if y % 2 == 0 { '▄' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(bg, (30, 26, 26), 0.5)), Some(rgb(bg)));
        }
    }
    let (m0, m1) = (f0 + 2, (f1 - 2).max(f0 + 2));
    for y in (g - mouth_h).max(0)..g {
        for x in m0..=m1 {
            let hs = hash(x * 13 + y, (t / 3) as i32);
            let up = (g - y) as f32 / mouth_h as f32;
            let lick = ((x as f32 * 1.7 + tf * 0.45).sin() * 0.5 + 0.5) * 0.55 + 0.35 + if pump { 0.15 } else { 0.0 };
            let bg = lerp((120, 30, 6), (240, 120, 30), flick - 0.6);
            if y == g - mouth_h && mouth_h > 1 {
                cv.put(x, y, '▄', rgb(bg), Some(rgb(lerp(stone, (0, 0, 0), 0.3))));
                continue;
            }
            if up > lick {
                cv.put(x, y, ' ', Color::Black, Some(rgb(lerp((60, 14, 4), bg, 0.5))));
                continue;
            }
            let col = [(255, 240, 150), (255, 170, 50), (240, 80, 20)][(hs % 3) as usize];
            let ch = if y == g - 1 { '▒' } else if hs % 3 == 0 { '^' } else { '▲' };
            cv.put(x, y, ch, rgb(col), Some(rgb(bg)));
        }
    }

    // The bellows, and the apprentice leaning on them.
    let bx = f1 + 1;
    let leather = (130, 78, 42);
    if pump {
        for x in bx..bx + 3 {
            cv.glyph(x, g - 1, '▄', rgb(leather));
        }
        cv.glyph(bx + 3, g - 2, '\\', rgb((150, 110, 70)));
    } else {
        for x in bx..bx + 3 {
            cv.glyph(x, g - 2, '▄', rgb(leather));
            cv.glyph(x, g - 1, '█', rgb(lerp(leather, (0, 0, 0), 0.2)));
        }
        cv.glyph(bx + 3, g - 2, '/', rgb((150, 110, 70)));
        cv.glyph(bx + 3, g - 3, '·', rgb((150, 110, 70)));
    }
    cv.glyph(bx - 1, g - 1, '═', rgb((110, 100, 90)));
    if helper {
        cv.glyph(bx + 4, g - 1 - if pump { 0 } else { 0 }, '☺', rgb((150, 200, 240)));
    }

    // A pile of coal before the furnace.
    for k in 0..(fw / 2).max(2) {
        for j in 0..=k.min(2) {
            let x = f0 + 1 + k;
            let y = g + 2 - j;
            if y > g && y < h {
                let ember = hash(x, y) % 5 == 0 && (t / 9 + x as u64) % 4 != 0;
                cv.glyph(x, y, if ember { '·' } else { '▲' }, rgb(if ember { (255, 110, 40) } else { (44, 40, 42) }));
            }
        }
    }

    // The anvil, the bar and the smith.
    let iron = rgb((92, 92, 104));
    let top_row = if big { "▄█████▀" } else { "▄███▀" };
    let base_row = if big { " ▐███▌ " } else { " ▐█▌ " };
    cv.sprite(ax, g - 2, &[top_row, base_row], iron);
    let blen = if big { 4 } else { 3 };
    for k in 0..blen {
        cv.glyph(ax + 1 + k, g - 3, '▄', rgb(bar));
    }
    let sx = ax - 2;
    cv.glyph(sx, g - 1, '☺', rgb(beard));
    let head = rgb((70, 70, 80));
    let wood = rgb((140, 100, 60));
    if p < 8 {
        cv.glyph(sx, g - 2, '│', wood);
        cv.glyph(sx, g - 3, '■', head);
    } else if p < 11 {
        cv.glyph(sx + 1, g - 2, '/', wood);
        cv.glyph(sx + 1, g - 3, '■', head);
    } else {
        cv.glyph(sx + 1, g - 2, '/', wood);
        cv.glyph(ax + 1, g - 3, '■', head);
    }
    // Light flashing off each blow, and the sparks it sends flying.
    let since = if p >= 11 { p - 11 } else { p + beat - 11 };
    let (ox, oy) = (ax + 2, g - 3);
    if since < 3 {
        cv.halo(ox, oy, 3.0 + heat * 2.0, hot, 0.5);
    }
    let n = if big { 18 } else { 10 };
    for j in 0..2u64 {
        let age = since + j * beat;
        if age > 30 || t < age {
            continue;
        }
        let strike = ((t - age) / beat) as i32;
        let a = age as f32;
        for k in 0..n {
            let hs = hash(k, strike);
            let vx = ((hs % 200) as f32 / 100.0 - 1.0) * 1.1;
            let vy = 0.3 + ((hs / 200) % 100) as f32 / 100.0 * 0.6;
            let x = ox + (vx * a * 0.8).round() as i32;
            let y = oy - (vy * a * 0.9 - 0.035 * a * a).round() as i32;
            if y >= g || a > 10.0 + (hs % 20) as f32 {
                continue;
            }
            let col = lerp(hot, (200, 50, 20), a / 30.0);
            cv.glyph(x, y, if a < 6.0 { '*' } else { '·' }, rgb(col));
        }
    }

    // Ingots cooling on the rack.
    let rx = ax + 8;
    let ningots = (w as f32 * 0.06).clamp(3.0, 12.0) as i32;
    let rw = ningots * 2 + 1;
    for x in rx..rx + rw {
        cv.glyph(x, g - 2, '═', rgb((110, 80, 50)));
    }
    cv.glyph(rx, g - 1, '║', rgb((110, 80, 50)));
    cv.glyph(rx + rw - 1, g - 1, '║', rgb((110, 80, 50)));
    let off = cv.roll(5) as u64;
    for i in 0..ningots {
        let age = ((t + off + i as u64 * 131) % 1000) as f32 / 1000.0;
        let heat = 1.0 - age;
        let col = if heat > 0.55 { lerp(warm, hot, (heat - 0.55) / 0.45) } else { lerp(cold, warm, heat / 0.55) };
        let x = rx + 1 + i * 2;
        cv.glyph(x, g - 3, '▄', rgb(col));
        if heat > 0.7 {
            cv.glow(x, g - 3, warm, (heat - 0.7) * 1.2);
            cv.glow(x, g - 4, warm, (heat - 0.7) * 0.6);
        }
    }

    // The quench trough, steaming.
    let qx = rx + rw + 3;
    cv.sprite(qx, g - 2, &["▌≈≈≈▐", "▀▀▀▀▀"], rgb((110, 80, 50)));
    for k in 1..4 {
        cv.glyph(qx + k, g - 2, '≈', rgb((90, 140, 200)));
    }
    let burst = (t % 420) < 50;
    for k in 0..if burst { 10 } else { 4 } {
        let age = ((t / 4) as i32 + k * 5) % 20;
        let x = qx + 2 + ((age as f32 * 0.3 + k as f32).sin() * 1.5) as i32;
        let y = g - 3 - age / 3;
        let fade = 1.0 - age as f32 / 20.0;
        cv.glyph(x, y, if age < 7 { '░' } else { '·' }, mix((50, 46, 50), (200, 196, 200), fade));
    }

    // Tools hung on the wall, and finished work on the rack.
    if g >= 7 {
        let ty = g - 5;
        let tools = ['┬', 'τ', '╥', 'Γ', '┬', '¬'];
        let mut x = ax - 1;
        let mut k = 0;
        while x < qx + 6 {
            cv.glyph(x, ty - 1, '·', rgb((70, 60, 56)));
            cv.glyph(x, ty, tools[k % tools.len()], rgb((120, 112, 110)));
            x += 3;
            k += 1;
        }
    }
    let wr0 = qx + 9;
    if g >= 5 && wr0 + 4 < arch_left - 3 {
        let heads = ['↑', '¥', 'Ψ', '♠', '↑', '†'];
        for x in wr0..arch_left - 3 {
            cv.glyph(x, g - 2, '═', rgb((110, 80, 50)));
        }
        let mut x = wr0 + 1;
        let mut k = 0;
        while x < arch_left - 4 {
            let shine = (t / 5 + x as u64 * 3) % 60 == 0;
            cv.glyph(x, g - 1, '│', rgb((120, 90, 60)));
            cv.glyph(x, g - 3, '│', rgb((120, 90, 60)));
            let steel = if shine { (255, 255, 240) } else { lerp(cold, (200, 200, 210), 0.4) };
            cv.glyph(x, g - 4, heads[(k + cv.roll(6) as usize) % heads.len()], rgb(steel));
            x += 3;
            k += 1;
        }
    }

    // Out on the floor: a grindstone at work, and the day's ore.
    if h - g >= 10 && w >= 60 {
        let (gx, gy) = (w - (w as f32 * 0.1) as i32 - 3, g + (h - g) / 2);
        let spin = ['|', '/', '─', '\\'][((t / 3) % 4) as usize];
        cv.sprite(gx - 1, gy - 1, &["▄█▄", "▀█▀"], rgb((120, 116, 110)));
        cv.glyph(gx, gy - 1, spin, rgb((60, 58, 56)));
        cv.glyph(gx, gy, '║', rgb((110, 80, 50)));
        cv.glyph(gx - 3, gy, '☺', rgb((235, 200, 120)));
        cv.glyph(gx - 2, gy - 1, '─', rgb(lerp(cold, (220, 220, 230), 0.5)));
        for k in 0..4 {
            let age = ((t / 2) as i32 + k * 3) % 10;
            let hs = hash(k, (t / 20) as i32);
            cv.glyph(gx - 1 - age, gy - 1 - age / 4 + (hs % 2) as i32, '·', rgb(lerp((255, 230, 140), (200, 70, 20), age as f32 / 10.0)));
        }
        let (ox, oy) = ((w as f32 * 0.1) as i32 + 4, h - 3);
        for k in 0..6 {
            for j in 0..=k.min(5 - k).min(2) {
                let x = ox + k;
                let y = oy - j;
                let fleck = hash(x, y) % 3 == 0;
                cv.glyph(x, y, if fleck { '*' } else { '▲' }, rgb(if fleck { lerp(cold, warm, 0.3) } else { (70, 64, 60) }));
            }
        }
        for k in 0..3 {
            cv.glyph(ox + 8 + k, oy, 'Θ', rgb((150, 100, 50)));
        }
        cv.glyph(ox + 9, oy - 1, 'Θ', rgb((150, 100, 50)));
    }

    // Sparks drifting up from the furnace mouth.
    for k in 0..5 {
        let age = ((t / 3) as i32 + k * 7) % 21;
        let x = mouth.0 + ((age as f32 * 0.4 + k as f32 * 2.0).sin() * 2.0) as i32;
        let y = g - mouth_h - 1 - age / 3;
        if y > ftop && age < 15 {
            cv.glyph(x, y, '·', rgb((255, 190, 80)));
        }
    }
}
