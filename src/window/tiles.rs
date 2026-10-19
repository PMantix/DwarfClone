//! Our own tileset, drawn in code rather than taken from a font: box-drawing
//! lines that run edge to edge so they join across cells, the block and shade
//! characters, and the map's water, ramps, trees and floor dots. Each comes
//! out as a coverage mask for one square cell (0 is all background, 255 all
//! foreground); the colours come from the cell when it's painted.

/// One cell's coverage, row by row, `s * s` bytes.
pub type Mask = Vec<u8>;

/// The drawn tile for `c` in a cell `s` pixels square, or None to take the
/// character from the font.
pub fn tile(c: char, s: usize) -> Option<Mask> {
    if let Some(arms) = box_arms(c) {
        return Some(box_lines(arms, s));
    }
    Some(match c {
        '╱' | '╲' | '╳' => diagonals(c, s),
        '\u{2580}'..='\u{259F}' => blocks(c, s),
        '≈' => waves(s),
        '▲' => ramp(s, true),
        '▼' => ramp(s, false),
        '♣' => leafy_tree(s),
        '♠' => conifer(s),
        '·' | '∙' => dot(s),
        _ => return None,
    })
}

// ------------------------------------------------------------ helpers

fn filled(s: usize, v: u8) -> Mask {
    vec![v; s * s]
}

/// Fill the pixels x0..x1, y0..y1 (clipped to the cell) with `v`.
fn rect(m: &mut Mask, s: usize, x0: i32, y0: i32, x1: i32, y1: i32, v: u8) {
    let c = |v: i32| v.clamp(0, s as i32) as usize;
    for y in c(y0)..c(y1) {
        for x in c(x0)..c(x1) {
            m[y * s + x] = v;
        }
    }
}

/// A shape given as "is this point inside", with 4x4 samples a pixel so its
/// edges are smooth. Points run 0..1 across the cell.
fn shape(s: usize, inside: impl Fn(f32, f32) -> bool) -> Mask {
    let mut m = filled(s, 0);
    let n = 4;
    for y in 0..s {
        for x in 0..s {
            let mut hits = 0;
            for sy in 0..n {
                for sx in 0..n {
                    let px = (x as f32 + (sx as f32 + 0.5) / n as f32) / s as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / n as f32) / s as f32;
                    hits += inside(px, py) as u32;
                }
            }
            m[y * s + x] = (hits * 255 / (n * n) as u32) as u8;
        }
    }
    m
}

/// Whether (x, y) is inside the triangle a, b, c.
fn in_triangle(x: f32, y: f32, a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let side = |p: (f32, f32), q: (f32, f32)| (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

fn in_circle(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> bool {
    (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r
}

// ------------------------------------------------------------ box drawing

const NONE: u8 = 0;
const LIGHT: u8 = 1;
const HEAVY: u8 = 2;
const DOUBLE: u8 = 3;

/// The arms of a box-drawing character, as [up, down, left, right], each
/// none, light, heavy or double.
fn box_arms(c: char) -> Option<[u8; 4]> {
    const L: u8 = LIGHT;
    const H: u8 = HEAVY;
    const D: u8 = DOUBLE;
    const N: u8 = NONE;
    Some(match c {
        '─' | '┄' | '┈' | '╌' => [N, N, L, L],
        '━' | '┅' | '┉' | '╍' => [N, N, H, H],
        '│' | '┆' | '┊' | '╎' => [L, L, N, N],
        '┃' | '┇' | '┋' | '╏' => [H, H, N, N],
        '┌' | '╭' => [N, L, N, L],
        '┐' | '╮' => [N, L, L, N],
        '└' | '╰' => [L, N, N, L],
        '┘' | '╯' => [L, N, L, N],
        '┏' => [N, H, N, H],
        '┓' => [N, H, H, N],
        '┗' => [H, N, N, H],
        '┛' => [H, N, H, N],
        '├' => [L, L, N, L],
        '┤' => [L, L, L, N],
        '┬' => [N, L, L, L],
        '┴' => [L, N, L, L],
        '┼' => [L, L, L, L],
        '┣' => [H, H, N, H],
        '┫' => [H, H, H, N],
        '┳' => [N, H, H, H],
        '┻' => [H, N, H, H],
        '╋' => [H, H, H, H],
        '╴' => [N, N, L, N],
        '╵' => [L, N, N, N],
        '╶' => [N, N, N, L],
        '╷' => [N, L, N, N],
        '╸' => [N, N, H, N],
        '╹' => [H, N, N, N],
        '╺' => [N, N, N, H],
        '╻' => [N, H, N, N],
        '╼' => [N, N, L, H],
        '╽' => [L, H, N, N],
        '╾' => [N, N, H, L],
        '╿' => [H, L, N, N],
        '═' => [N, N, D, D],
        '║' => [D, D, N, N],
        '╒' => [N, L, N, D],
        '╓' => [N, D, N, L],
        '╔' => [N, D, N, D],
        '╕' => [N, L, D, N],
        '╖' => [N, D, L, N],
        '╗' => [N, D, D, N],
        '╘' => [L, N, N, D],
        '╙' => [D, N, N, L],
        '╚' => [D, N, N, D],
        '╛' => [L, N, D, N],
        '╜' => [D, N, L, N],
        '╝' => [D, N, D, N],
        '╞' => [L, L, N, D],
        '╟' => [D, D, N, L],
        '╠' => [D, D, N, D],
        '╡' => [L, L, D, N],
        '╢' => [D, D, L, N],
        '╣' => [D, D, D, N],
        '╤' => [N, L, D, D],
        '╥' => [N, D, L, L],
        '╦' => [N, D, D, D],
        '╧' => [L, N, D, D],
        '╨' => [D, N, L, L],
        '╩' => [D, N, D, D],
        '╪' => [L, L, D, D],
        '╫' => [D, D, L, L],
        '╬' => [D, D, D, D],
        _ => return None,
    })
}

/// Line thicknesses and offsets for a cell size, the same in every cell so
/// lines meet exactly where the neighbouring cell's lines start.
struct Lines {
    s: i32,
    light: i32,
    /// One stroke of a double line.
    thin: i32,
    /// How far each stroke of a double line sits from the middle.
    gap: i32,
}

impl Lines {
    fn new(s: usize) -> Lines {
        let s = s as i32;
        let light = ((s + 4) / 10).max(1);
        let thin = (s / 16).max(1);
        Lines { s, light, thin, gap: (s * 3 / 16).max(thin + 1) }
    }

    fn thickness(&self, kind: u8) -> i32 {
        if kind == HEAVY {
            self.light * 2
        } else {
            self.light
        }
    }

    /// The span of a single line of thickness `t` across the middle.
    fn mid(&self, t: i32) -> (i32, i32) {
        let lo = (self.s - t) / 2;
        (lo, lo + t)
    }

    /// The outer edges of a double line's two strokes.
    fn band(&self) -> (i32, i32) {
        let (lo, hi) = self.mid(self.thin);
        (lo - self.gap, hi + self.gap)
    }

    /// The space between a double line's two strokes.
    fn channel(&self) -> (i32, i32) {
        let (lo, hi) = self.band();
        (lo + self.thin, hi - self.thin)
    }
}

/// Draw a box-drawing character. Arms run from the cell's edge to the middle,
/// so a line carries on unbroken into the next cell. Double lines are drawn
/// as a solid band with the space between its strokes cut out afterwards,
/// which joins corners and crossings the way they should look.
fn box_lines(arms: [u8; 4], s: usize) -> Mask {
    let mut m = filled(s, 0);
    let l = Lines::new(s);
    let si = s as i32;
    let half = si / 2;
    // Arm 0 up, 1 down, 2 left, 3 right; its perpendicular arms and its opposite.
    let perp = |i: usize| if i < 2 { [arms[2], arms[3]] } else { [arms[0], arms[1]] };
    let opposite = |i: usize| arms[i ^ 1];
    // The thickest single line crossing this arm, if any.
    let perp_single = |i: usize| perp(i).iter().filter(|&&k| k == LIGHT || k == HEAVY).map(|&k| l.thickness(k)).max();
    let perp_double = |i: usize| perp(i).contains(&DOUBLE);
    // Paint arm `i` covering `across` (lo..hi) and reaching from its edge to
    // `near` (towards the middle: the end of an up/left arm, the start of a
    // down/right one).
    let arm = |m: &mut Mask, i: usize, across: (i32, i32), near: i32, v: u8| match i {
        0 => rect(m, s, across.0, 0, across.1, near, v),
        1 => rect(m, s, across.0, near, across.1, si, v),
        2 => rect(m, s, 0, across.0, near, across.1, v),
        _ => rect(m, s, near, across.0, si, across.1, v),
    };
    let toward_start = |i: usize| i == 0 || i == 2;
    let (band, chan) = (l.band(), l.channel());
    // Double lines: the band...
    for i in 0..4 {
        if arms[i] != DOUBLE {
            continue;
        }
        let near = if perp_double(i) {
            if toward_start(i) { band.1 } else { band.0 }
        } else if let Some(t) = perp_single(i) {
            let (lo, hi) = l.mid(t);
            if toward_start(i) { hi } else { lo }
        } else {
            half
        };
        arm(&mut m, i, band, near, 255);
    }
    // ...then the space between its strokes.
    for i in 0..4 {
        if arms[i] != DOUBLE {
            continue;
        }
        let near = if perp_double(i) {
            if toward_start(i) { chan.1 } else { chan.0 }
        } else if let Some(t) = perp_single(i) {
            let (lo, hi) = l.mid(t);
            if toward_start(i) { lo } else { hi }
        } else {
            half
        };
        arm(&mut m, i, chan, near, 0);
    }
    // Single lines, light or heavy.
    for i in 0..4 {
        if arms[i] != LIGHT && arms[i] != HEAVY {
            continue;
        }
        let t = l.thickness(arms[i]);
        let (lo, hi) = l.mid(t);
        let near = if opposite(i) == NONE && perp_double(i) {
            // Meets the nearer stroke of a double line, not crossing the gap.
            if toward_start(i) { chan.0 } else { chan.1 }
        } else {
            // Through the middle, so a corner's outside is square.
            let w = perp_single(i).unwrap_or(t).max(t);
            let (plo, phi) = l.mid(w);
            if toward_start(i) { phi } else { plo }
        };
        arm(&mut m, i, (lo, hi), near, 255);
    }
    m
}

/// ╱ ╲ ╳: corner to corner, so they join diagonally.
fn diagonals(c: char, s: usize) -> Mask {
    let w = Lines::new(s).light as f32 / s as f32 * 0.75;
    let up = c != '╲';
    let down = c != '╱';
    shape(s, |x, y| (up && (x + y - 1.0).abs() < w) || (down && (x - y).abs() < w))
}

// ------------------------------------------------------------ blocks and shades

/// The block elements, U+2580..U+259F: halves, eighths, quadrants and shades.
fn blocks(c: char, s: usize) -> Mask {
    let mut m = filled(s, 0);
    let si = s as i32;
    let h = si / 2;
    let eighth = |k: i32| (si * k + 4) / 8;
    match c {
        '▀' => rect(&mut m, s, 0, 0, si, h, 255),
        '\u{2581}'..='\u{2587}' => {
            let k = c as i32 - 0x2580;
            rect(&mut m, s, 0, si - eighth(k), si, si, 255)
        }
        '█' => rect(&mut m, s, 0, 0, si, si, 255),
        '\u{2589}'..='\u{258F}' => {
            let k = 8 - (c as i32 - 0x2588);
            rect(&mut m, s, 0, 0, eighth(k), si, 255)
        }
        '▐' => rect(&mut m, s, h, 0, si, si, 255),
        '░' | '▒' => {
            // Stipple at a quarter or a half, on a grid that tiles across cells.
            let u = if s >= 32 { 2 } else { 1 };
            for y in 0..s {
                for x in 0..s {
                    let (a, b) = (x / u, y / u);
                    let on = if c == '░' { a % 2 == 0 && b % 2 == 0 } else { (a + b) % 2 == 0 };
                    m[y * s + x] = if on { 255 } else { 0 };
                }
            }
        }
        '▓' => {
            // Dressed stone: two courses of blocks a cell, the joints staggered.
            let u = if s >= 24 { 2 } else { 1 };
            rect(&mut m, s, 0, 0, si, si, 255);
            for course in 0..2 {
                let y0 = course * h;
                rect(&mut m, s, 0, y0 + h - u, si, y0 + h, 0);
                let jx = if course == 0 { 0 } else { h };
                rect(&mut m, s, jx, y0, jx + u, y0 + h, 0);
            }
        }
        '▔' => rect(&mut m, s, 0, 0, si, eighth(1), 255),
        '▕' => rect(&mut m, s, si - eighth(1), 0, si, si, 255),
        _ => {
            // Quadrants: upper left, upper right, lower left, lower right.
            let q: [bool; 4] = match c {
                '▖' => [false, false, true, false],
                '▗' => [false, false, false, true],
                '▘' => [true, false, false, false],
                '▙' => [true, false, true, true],
                '▚' => [true, false, false, true],
                '▛' => [true, true, true, false],
                '▜' => [true, true, false, true],
                '▝' => [false, true, false, false],
                '▞' => [false, true, true, false],
                '▟' => [false, true, true, true],
                '▌' => [true, false, true, false],
                '▄' => [false, false, true, true],
                _ => [false; 4],
            };
            for (i, on) in q.iter().enumerate() {
                if *on {
                    let (x0, y0) = ((i as i32 % 2) * h, (i as i32 / 2) * h);
                    let (x1, y1) = (if i % 2 == 0 { h } else { si }, if i < 2 { h } else { si });
                    rect(&mut m, s, x0, y0, x1, y1, 255);
                }
            }
        }
    }
    m
}

// ------------------------------------------------------------ the map

/// ≈ water and magma: two rows of waves, one wavelength a cell, so a lake's
/// surface rolls on unbroken from cell to cell.
fn waves(s: usize) -> Mask {
    let tau = std::f32::consts::TAU;
    let amp = 0.09;
    let half = 0.065_f32.max(0.6 / s as f32);
    shape(s, |x, y| {
        let a = 0.30 + amp * (tau * x).sin();
        let b = 0.74 + amp * (tau * x + std::f32::consts::PI).sin();
        (y - a).abs() < half || (y - b).abs() < half
    })
}

/// ▲ ▼: a ramp up, or the top of one seen from above. Smaller than the cell
/// so a hillside of ramps still shows the ground between them.
fn ramp(s: usize, up: bool) -> Mask {
    let (top, bot) = (0.2, 0.8);
    let (apex, base) = if up { (top, bot) } else { (bot, top) };
    shape(s, |x, y| in_triangle(x, y, (0.5, apex), (0.14, base), (0.86, base)))
}

/// ♣: a broad-leaved tree, round crown on a short trunk.
fn leafy_tree(s: usize) -> Mask {
    shape(s, |x, y| {
        let crown = in_circle(x, y, 0.5, 0.30, 0.24)
            || in_circle(x, y, 0.28, 0.50, 0.22)
            || in_circle(x, y, 0.72, 0.50, 0.22)
            || in_circle(x, y, 0.5, 0.48, 0.22);
        let trunk = (0.43..0.57).contains(&x) && (0.55..0.95).contains(&y);
        crown || trunk
    })
}

/// ♠: a conifer, three tiers of boughs.
fn conifer(s: usize) -> Mask {
    shape(s, |x, y| {
        let tier = |apex: f32, base: f32, w: f32| in_triangle(x, y, (0.5, apex), (0.5 - w, base), (0.5 + w, base));
        tier(0.04, 0.40, 0.24) || tier(0.20, 0.62, 0.34) || tier(0.38, 0.84, 0.44) || ((0.44..0.56).contains(&x) && (0.8..0.96).contains(&y))
    })
}

/// · a floor dot, in the middle of the cell.
fn dot(s: usize) -> Mask {
    let r = (0.1_f32).max(1.1 / s as f32);
    shape(s, |x, y| in_circle(x, y, 0.5, 0.5, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(m: &Mask, s: usize, x: usize, y: usize) -> u8 {
        m[y * s + x]
    }

    #[test]
    fn horizontal_lines_reach_both_edges() {
        for s in [8, 16, 22, 32] {
            for c in ['─', '━', '═', '┼', '╬', '╦', '╪'] {
                let m = tile(c, s).unwrap();
                // Some row is lit from the very first column to the very last.
                let joined = (0..s).any(|y| at(&m, s, 0, y) == 255 && at(&m, s, s - 1, y) == 255);
                assert!(joined, "{} at {}px doesn't reach both edges", c, s);
            }
        }
    }

    #[test]
    fn lines_meet_their_neighbours_exactly() {
        // A line's rows at the right edge of one cell are the rows at the left
        // edge of the next, whichever box character each cell holds.
        let s = 16;
        let rows = |c: char, x: usize| -> Vec<usize> {
            let m = tile(c, s).unwrap();
            (0..s).filter(|&y| at(&m, s, x, y) == 255).collect()
        };
        assert_eq!(rows('─', s - 1), rows('┐', 0));
        assert_eq!(rows('┌', s - 1), rows('─', 0));
        assert_eq!(rows('┼', s - 1), rows('┤', 0));
        assert_eq!(rows('═', s - 1), rows('╗', 0));
        assert_eq!(rows('╔', s - 1), rows('╦', 0));
        assert_eq!(rows('═', s - 1).len(), 2, "a double line is two strokes");
        // And the same for columns down the cells.
        let cols = |c: char, y: usize| -> Vec<usize> {
            let m = tile(c, s).unwrap();
            (0..s).filter(|&x| at(&m, s, x, y) == 255).collect()
        };
        assert_eq!(cols('│', s - 1), cols('└', 0));
        assert_eq!(cols('║', s - 1), cols('╚', 0));
        assert_eq!(cols('╦', s - 1), cols('║', 0));
    }

    #[test]
    fn double_corner_is_two_nested_corners() {
        let s = 16;
        let m = tile('╔', s).unwrap();
        // Nothing above or left of the corner, nothing in the gap.
        assert_eq!(at(&m, s, 0, 0), 0);
        assert_eq!(at(&m, s, s / 2, s / 2), 0, "the inside of the corner is empty");
        // Both strokes leave by the right edge and by the bottom.
        assert_eq!((0..s).filter(|&y| at(&m, s, s - 1, y) == 255).count(), 2);
        assert_eq!((0..s).filter(|&x| at(&m, s, x, s - 1) == 255).count(), 2);
    }

    #[test]
    fn solid_wall_and_water_fill_the_cell() {
        let s = 16;
        assert!(tile('█', s).unwrap().iter().all(|&v| v == 255));
        // Waves run off both sides at the same height, so lakes join up.
        let w = tile('≈', s).unwrap();
        for y in 0..s {
            if at(&w, s, 0, y) > 200 {
                let near = (y.saturating_sub(1)..(y + 2).min(s)).any(|y2| at(&w, s, s - 1, y2) > 0);
                assert!(near, "row {} of the wave doesn't carry on into the next cell", y);
            }
        }
        assert!((0..s).any(|y| at(&w, s, 0, y) > 100));
    }

    #[test]
    fn text_comes_from_the_font() {
        assert!(tile('a', 16).is_none());
        assert!(tile('<', 16).is_none());
        assert!(tile('☺', 16).is_none());
    }
}
