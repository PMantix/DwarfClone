//! Rough pictures of the backdrops, for looking at them without a
//! terminal: each cell becomes an 8x16 block of its background with a
//! crude shape of its glyph on top, written out as a PNG.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

const CW: usize = 8;
const CH: usize = 16;

fn rgb(c: Color, dflt: [u8; 3]) -> [u8; 3] {
    match c {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => dflt,
    }
}

/// Whether pixel (px, py) of a cell is inked for glyph `ch`.
fn ink(ch: char, px: usize, py: usize) -> bool {
    let (x, y) = (px as i32, py as i32);
    let (cx, cy) = (CW as i32 / 2, CH as i32 / 2);
    match ch {
        ' ' => false,
        '█' => true,
        '▀' => y < cy,
        '▄' => y >= cy,
        '▌' => x < cx,
        '▐' => x >= cx,
        '▓' => (x + y) % 4 != 0,
        '▒' => (x + y) % 2 == 0,
        '░' => x % 2 == 0 && y % 2 == 0,
        '│' | '║' => (x - cx).abs() <= 1,
        '─' | '═' | '▬' => (y - cy).abs() <= 1,
        '╥' | '┬' => (y - cy).abs() <= 1 || ((x - cx).abs() <= 1 && y > cy),
        '┼' => (y - cy + 2).abs() <= 1 || (x - cx).abs() <= 1,
        '·' => (x - cx).abs() <= 1 && (y - cy).abs() <= 1,
        '.' | ',' => (x - cx).abs() <= 1 && y >= CH as i32 - 4,
        '\'' | '`' | '"' => (x - cx).abs() <= 1 && y < 5,
        '/' | '╱' => (x - (CW as i32 - 1 - y * CW as i32 / CH as i32)).abs() <= 1,
        '\\' | '╲' => (x - y * CW as i32 / CH as i32).abs() <= 1,
        '▲' => y >= 4 && (x - cx).abs() * 3 <= (y - 4),
        '▼' => y < 12 && (x - cx).abs() * 3 <= (12 - y),
        '~' | '≈' => (y - cy - ((x as f32 * 0.9).sin() * 2.0) as i32).abs() <= 1,
        '*' | '+' => (x - cx).abs() <= 1 && (y - cy).abs() <= 3 || (y - cy).abs() <= 1 && (x - cx).abs() <= 3,
        _ => {
            let (dx, dy) = ((x - cx) as f32 / 3.5, (y - cy) as f32 / 6.0);
            dx * dx + dy * dy <= 1.0
        }
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// Encode RGB pixels as a PNG (uncompressed deflate; it's only for looking).
fn png(w: usize, h: usize, px: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(h * (w * 3 + 1));
    for row in px.chunks(w * 3) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(if i + 1 == blocks.len() { 1 } else { 0 });
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Write `buf` as a picture at `path`.
pub fn save(buf: &Buffer, path: &std::path::Path) {
    let (w, h) = (buf.area.width as usize, buf.area.height as usize);
    let (pw, ph) = (w * CW, h * CH);
    let mut px = vec![0u8; pw * ph * 3];
    for cy in 0..h {
        for cx in 0..w {
            let cell = buf.get(cx as u16, cy as u16);
            let bg = rgb(cell.bg, [0, 0, 0]);
            let fg = rgb(cell.fg, [200, 200, 200]);
            let ch = cell.symbol().chars().next().unwrap_or(' ');
            for y in 0..CH {
                for x in 0..CW {
                    let c = if ink(ch, x, y) { fg } else { bg };
                    let i = ((cy * CH + y) * pw + cx * CW + x) * 3;
                    px[i..i + 3].copy_from_slice(&c);
                }
            }
        }
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, png(pw, ph, &px)).unwrap();
}
