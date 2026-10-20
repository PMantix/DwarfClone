//! How a bare tile looks: its glyph and colours, the season's trees and
//! grass, water and magma (M2 W0-B: moved verbatim out of ui.rs, so the
//! highlight reel's export (O05) and later map styles edit this file).
//! Units, items and buildings on a tile are drawn over it by `draw_map`.

use super::{dim, hash2, BLACK};
use crate::geom::Pos;
use crate::items::ItemKind;
use crate::material::Mat;
use crate::world::{TileKind, World};
use ratatui::style::Color;

/// The season's look for trees and grass under the open sky: blossom in
/// spring, autumn colours, bare branches in winter, fallen leaves.
/// `year_frac` runs 0..1 through the year from the first day of spring.
pub(super) fn foliage(w: &World, p: Pos, year_frac: f32, cold: bool) -> Option<(char, Color)> {
    use crate::overworld::Biome as B;
    let t = w.t(p);
    if !t.revealed || !w.outside(p) {
        return None;
    }
    let h = hash2(p.x * 3 + 11, p.y * 5 + 7);
    // What grows here depends on the land: cacti in the desert, conifers in
    // the taiga, acacias on the savanna, dense green in the jungle, and
    // twisted, sickly growth where evil lies on the land.
    let (biome, evil) = w.biome_at(p.x, p.y).unwrap_or((B::Forest, false));
    let cold = cold || biome.cold();
    let pick = |opts: &[(char, (u8, u8, u8))]| {
        let (c, col) = opts[(h as usize / 5) % opts.len()];
        (c, Color::Rgb(col.0, col.1, col.2))
    };
    if evil && matches!(t.kind, TileKind::Tree | TileKind::Shrub) {
        return Some(pick(&[('♣', (120, 95, 125)), ('¥', (105, 85, 110)), ('♠', (95, 80, 100))]));
    }
    if t.kind == TileKind::Shrub {
        return Some(match biome {
            B::Desert | B::Badlands => pick(&[('*', (170, 150, 90)), ('"', (150, 140, 80))]),
            B::Swamp => pick(&[('"', (120, 160, 90)), ('|', (110, 140, 80))]),
            B::Jungle => pick(&[('"', (40, 190, 70)), ('♣', (60, 170, 60))]),
            B::Tundra | B::Glacier | B::Taiga => pick(&[('"', (110, 140, 120)), ('·', (130, 150, 130))]),
            B::Savanna => pick(&[('"', (180, 165, 70)), ('*', (160, 150, 70))]),
            _ => ('"', Color::Rgb(90, 190, 90)),
        });
    }
    if t.kind == TileKind::Tree {
        match biome {
            B::Desert | B::Badlands => return Some(pick(&[('Ψ', (90, 165, 95)), ('Ψ', (110, 175, 100)), ('¥', (150, 130, 90))])),
            B::Jungle => return Some(pick(&[('♠', (25, 120, 45)), ('♣', (35, 165, 55)), ('Γ', (60, 150, 60)), ('♣', (20, 140, 70))])),
            B::Savanna => return Some(pick(&[('τ', (150, 150, 60)), ('τ', (130, 140, 55)), ('♣', (120, 140, 60))])),
            B::Taiga | B::Tundra | B::Glacier => return Some(pick(&[('♠', (35, 105, 65)), ('↑', (45, 115, 75)), ('♠', (55, 125, 85))])),
            B::Swamp => return Some(pick(&[('♣', (60, 120, 90)), ('♣', (70, 110, 70)), ('♠', (50, 100, 80))])),
            _ => {}
        }
    }
    // Each tree turns a little ahead of or behind its neighbours.
    let jitter = (h % 100) as f32 / 100.0 * 0.06 - 0.03;
    let f = (year_frac + jitter).rem_euclid(1.0);
    let evergreen = if cold { h % 4 != 3 } else { h % 4 == 0 };
    match t.kind {
        TileKind::Tree if evergreen => Some(('♠', Color::Rgb(30, 120, 60))),
        TileKind::Tree => Some(match f {
            // Early spring: some trees blossom, the rest bud pale green.
            f if f < 0.14 => {
                if h % 3 == 0 {
                    ('♣', [Color::Rgb(255, 175, 205), Color::Rgb(245, 235, 240)][(h / 3 % 2) as usize])
                } else {
                    ('♣', Color::Rgb(150, 215, 110))
                }
            }
            f if f < 0.53 => ('♣', Color::Rgb(40, 170, 50)),
            // Autumn: yellow, orange, red and gold, then bare.
            f if f < 0.70 => (
                '♣',
                [Color::Rgb(235, 205, 60), Color::Rgb(235, 135, 40), Color::Rgb(205, 60, 40), Color::Rgb(205, 165, 50)][(h % 4) as usize],
            ),
            f if f < 0.97 => ('♣', Color::Rgb(125, 95, 70)),
            _ => ('♣', Color::Rgb(150, 215, 110)),
        }),
        // Leaves lie under the trees from late autumn until the snow.
        TileKind::Floor if t.mat == Mat::Grass && !t.snow && (0.62..0.85).contains(&f) && h % 3 != 0 => {
            let near = crate::geom::DIRS8.iter().any(|&(dx, dy)| w.kind(p.offset(dx, dy, 0)) == Some(TileKind::Tree));
            near.then(|| {
                (
                    [',', '\'', '`'][(h % 3) as usize],
                    [Color::Rgb(200, 120, 40), Color::Rgb(180, 70, 40), Color::Rgb(210, 175, 60)][(h / 7 % 3) as usize],
                )
            })
        }
        _ => None,
    }
}

/// Appearance of a bare tile (no units, items or buildings).
pub(crate) fn tile_look(w: &World, p: Pos) -> (char, Color, Color) {
    let t = w.t(p);
    if !t.revealed {
        return (' ', BLACK, BLACK);
    }
    let mc = t.mat.color();
    let water_bg = Color::Rgb(0, 30, 90);
    let water_fg = Color::Rgb(90, 160, 255);
    let magma_bg = Color::Rgb(120, 20, 0);
    let magma_fg = Color::Rgb(255, 150, 40);
    if t.magma > 0 {
        return (water_glyph(t.magma), magma_fg, magma_bg);
    }
    if t.frozen && t.water > 0 {
        return ('≈', Color::Rgb(220, 240, 255), Color::Rgb(70, 100, 130));
    }
    let (g, fg, bg) = match t.kind {
        TileKind::Open => {
            if t.water > 0 {
                return (water_glyph(t.water), water_fg, water_bg);
            }
            // Look down through open space.
            for k in 1..=14 {
                let q = p.offset(0, 0, -k);
                let Some(tq) = w.get(q) else { break };
                if !tq.revealed {
                    break;
                }
                let f = (0.62 - 0.035 * k as f32).max(0.22);
                if tq.water > 0 {
                    return ('≈', dim(water_fg, f), BLACK);
                }
                if tq.magma > 0 {
                    return ('≈', dim(magma_fg, f.max(0.45)), BLACK);
                }
                let c = dim(tq.mat.color(), f);
                return match tq.kind {
                    TileKind::Open => continue,
                    TileKind::Ramp => ('▼', c, BLACK),
                    TileKind::Tree => ('♣', c, BLACK),
                    TileKind::StairUp | TileKind::StairUD => ('<', c, BLACK),
                    _ => ('·', c, BLACK),
                };
            }
            (' ', BLACK, BLACK)
        }
        TileKind::Floor if t.built_from.is_some() => ('+', dim(mc, 0.85), BLACK),
        TileKind::Wall if t.smooth >= crate::game::ENGRAVED => ('▒', mc, Color::Rgb(25, 22, 30)),
        TileKind::Wall if t.smooth == 1 => ('█', mc, BLACK),
        TileKind::Floor if t.smooth >= crate::game::ENGRAVED => ('÷', mc, BLACK),
        TileKind::Floor if t.smooth == 1 => ('·', mc, BLACK),
        TileKind::Wall if t.built_from.is_some() => {
            if t.built_from == Some(ItemKind::Block) {
                ('▓', mc, BLACK)
            } else {
                ('█', mc, BLACK)
            }
        }
        TileKind::Fortification => ('#', mc, BLACK),
        TileKind::Boulder => ('●', mc, BLACK),
        TileKind::Floor => {
            use crate::overworld::Biome as B;
            let h = hash2(p.x, p.y);
            let shade = 0.75 + (h % 7) as f32 * 0.05;
            if t.mat == Mat::Grass {
                // Grass by its land: tall and golden on the savanna, short and
                // pale on the tundra, dark under the pines, sickly where evil lies.
                let (biome, evil) = w.biome_at(p.x, p.y).unwrap_or((B::Grassland, false));
                let (glyphs, (r, gg, b)): (&[char], (u8, u8, u8)) = match biome {
                    B::Savanna => (&['"', '"', ',', '\'', '"'], (190, 172, 80)),
                    B::Tundra => (&['.', ',', '`', '.', '\''], (150, 166, 142)),
                    B::Taiga => (&[',', '\'', '.', ',', '`'], (72, 132, 88)),
                    B::Jungle => (&['"', ',', '"', '\'', '"'], (40, 168, 72)),
                    B::Swamp => (&['"', ',', '"', '.', '\''], (88, 138, 106)),
                    B::Mountain | B::Hills => (&[',', '.', '`', '\'', ','], (112, 152, 90)),
                    B::Desert | B::Badlands => (&['.', ',', '`', '.', '\''], (172, 160, 92)),
                    _ => (&[',', '.', '`', '\'', ','], (70, 160, 60)),
                };
                let (r, gg, b) = if evil { (126, 104, 136) } else { (r, gg, b) };
                (glyphs[(h % 5) as usize], dim(Color::Rgb(r, gg, b), shade), BLACK)
            } else if w.outside(p) && matches!(t.mat, Mat::Sand | Mat::Clay) {
                // Dunes and dry earth under the sky.
                let g = if t.mat == Mat::Sand { ['.', '~', '.', ',', '~', '.'][(h % 6) as usize] } else { ['.', ',', '.', '`', '.', '_'][(h % 6) as usize] };
                (g, dim(mc, shade), BLACK)
            } else {
                ('.', dim(mc, 0.8), BLACK)
            }
        }
        TileKind::Wall => {
            if t.mat == Mat::Bedrock {
                ('▓', mc, BLACK)
            } else if t.mat.is_gem() {
                ('☼', mc, Color::Rgb(40, 20, 50))
            } else if t.mat.is_ore() {
                ('£', mc, Color::Rgb(35, 30, 30))
            } else {
                ('█', dim(mc, 0.75), BLACK)
            }
        }
        TileKind::Ramp => ('▲', mc, BLACK),
        TileKind::StairUp => ('<', mc, BLACK),
        TileKind::StairDown => ('>', mc, BLACK),
        TileKind::StairUD => ('X', mc, BLACK),
        TileKind::Tree => {
            if w.outside(p) {
                ('♣', Color::Rgb(40, 170, 50), BLACK)
            } else {
                ('♠', Color::Rgb(170, 120, 210), BLACK)
            }
        }
        TileKind::Shrub => ('"', Color::Rgb(90, 190, 90), BLACK),
    };
    if t.water > 0 {
        return (water_glyph(t.water), water_fg, water_bg);
    }
    if t.aquifer && t.kind == TileKind::Wall {
        return (g, fg, Color::Rgb(10, 30, 70));
    }
    (g, fg, bg)
}

fn water_glyph(level: u8) -> char {
    if level >= 4 {
        '≈'
    } else {
        (b'0' + level) as char
    }
}
