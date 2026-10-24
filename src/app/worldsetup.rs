//! The world creation settings: what each row of the screen sets, how it
//! reads, and the keys that change it. ↑↓ choose a row, ←→ change it,
//! `+`/`-` fine-tune the world's width on the size row, digits type a seed,
//! `r` rolls a new seed, `d` puts every setting back to the usual.

use crate::overworld::WorldParams;

/// One row of the settings screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Preset,
    Seed,
    Size,
    History,
    Civs,
    Spacing,
    Epics,
    Savagery,
    Minerals,
    Ocean,
    Mountains,
    Rain,
    Heat,
    Forest,
    Generate,
}

/// The rows in screen order, with the group heading that goes above some.
pub const ROWS: [(Row, Option<&str>); 15] = [
    (Row::Preset, None),
    (Row::Seed, None),
    (Row::Size, Some("The world")),
    (Row::History, None),
    (Row::Civs, Some("Its peoples")),
    (Row::Spacing, None),
    (Row::Epics, None),
    (Row::Savagery, None),
    (Row::Minerals, Some("The land")),
    (Row::Ocean, None),
    (Row::Mountains, None),
    (Row::Rain, None),
    (Row::Heat, None),
    (Row::Forest, None),
    (Row::Generate, None),
];

pub const HISTORY_CHOICES: [i32; 7] = [25, 50, 100, 200, 300, 500, 1000];

/// The size presets in the order ←→ steps through them (by `size` value).
const SIZE_ORDER: [u8; 5] = [4, 0, 1, 2, 3];
const SIZE_NAMES: [&str; 5] = ["Small", "Medium", "Large", "Huge", "Tiny"];

/// Five-step levels for the land's make-up.
const FIVE: [[&str; 5]; 5] = [
    ["Little", "Less", "Normal", "More", "Much"],
    ["Flat", "Gentle", "Normal", "Rugged", "Towering"],
    ["Arid", "Dry", "Normal", "Wet", "Drenched"],
    ["Frigid", "Cold", "Normal", "Warm", "Hot"],
    ["Bare", "Sparse", "Normal", "Wooded", "Overgrown"],
];

/// (name, history, civs, savagery, minerals, spacing, epics)
pub const PRESETS: [(&str, i32, u8, u8, u8, u8, u8); 5] = [
    ("Quick", 50, 1, 1, 1, 1, 1),
    ("Standard", 200, 2, 1, 1, 1, 1),
    ("Long history", 500, 3, 1, 1, 1, 1),
    ("Harsh", 300, 2, 2, 0, 1, 2),
    ("Lonely", 300, 0, 1, 1, 2, 1),
];

pub fn preset_name(p: &WorldParams) -> &'static str {
    PRESETS
        .iter()
        .find(|x| (x.1, x.2, x.3, x.4, x.5, x.6) == (p.history, p.civs, p.savagery, p.minerals, p.spacing, p.epics))
        .map_or("Custom", |x| x.0)
}

impl Row {
    pub fn label(self) -> &'static str {
        match self {
            Row::Preset => "Preset",
            Row::Seed => "Seed",
            Row::Size => "World size",
            Row::History => "History",
            Row::Civs => "Civilizations",
            Row::Spacing => "Site spacing",
            Row::Epics => "Great events",
            Row::Savagery => "Savagery",
            Row::Minerals => "Minerals",
            Row::Ocean => "Ocean",
            Row::Mountains => "Mountains",
            Row::Rain => "Rainfall",
            Row::Heat => "Temperature",
            Row::Forest => "Forests",
            Row::Generate => "Generate world",
        }
    }

    /// What the row is set to, as the screen shows it.
    pub fn value(self, p: &WorldParams) -> String {
        match self {
            Row::Preset => preset_name(p).to_string(),
            Row::Seed => format!("{}", p.seed),
            Row::Size => {
                let (w, h) = p.dims();
                let name = if p.width > 0 { "Custom" } else { SIZE_NAMES[p.size.min(4) as usize] };
                format!("{} {}×{}", name, w, h)
            }
            Row::History => format!("{} years", p.history),
            Row::Civs => ["Very few", "Few", "Normal", "Many", "Crowded"][p.civs.min(4) as usize].to_string(),
            Row::Spacing => ["Close", "Normal", "Far apart"][p.spacing.min(2) as usize].to_string(),
            Row::Epics => ["None", "Rare", "Common"][p.epics.min(2) as usize].to_string(),
            Row::Savagery => ["Calm", "Normal", "Savage"][p.savagery.min(2) as usize].to_string(),
            Row::Minerals => ["Sparse", "Normal", "Rich"][p.minerals.min(2) as usize].to_string(),
            Row::Ocean => FIVE[0][p.ocean.min(4) as usize].to_string(),
            Row::Mountains => FIVE[1][p.mountains.min(4) as usize].to_string(),
            Row::Rain => FIVE[2][p.rain.min(4) as usize].to_string(),
            Row::Heat => FIVE[3][p.heat.min(4) as usize].to_string(),
            Row::Forest => FIVE[4][p.forest.min(4) as usize].to_string(),
            Row::Generate => String::new(),
        }
    }

    /// A short note on what the row does.
    pub fn note(self) -> &'static str {
        match self {
            Row::Preset => "Quick, Standard, Long, Harsh, Lonely",
            Row::Seed => "type digits · r random",
            Row::Size => "+/- fine-tune the width",
            Row::History => "years simulated before you arrive",
            Row::Civs => "how many peoples share the world",
            Row::Spacing => "how much room each settlement keeps",
            Row::Epics => "cataclysms, plagues, new empires",
            Row::Savagery => "dangerous wildlife and great beasts",
            Row::Minerals => "how much ore and gem lies in the rock",
            Row::Ocean => "how much of the world is sea",
            Row::Mountains => "how much is hills and peaks",
            Row::Rain => "dry deserts or wet jungles and marshes",
            Row::Heat => "ice and tundra or savanna and jungle",
            Row::Forest => "how readily the land grows trees",
            Row::Generate => "",
        }
    }

    /// Change the setting one step left or right. Returns false for rows
    /// that don't step (the seed rolls instead).
    pub fn step(self, p: &mut WorldParams, right: bool) {
        let lvl = |v: u8, max: u8| if right { (v + 1).min(max) } else { v.saturating_sub(1) };
        match self {
            Row::Preset => {
                let cur = PRESETS.iter().position(|x| x.0 == preset_name(p));
                let i = match (cur, right) {
                    (Some(i), true) => (i + 1) % PRESETS.len(),
                    (Some(i), false) => (i + PRESETS.len() - 1) % PRESETS.len(),
                    (None, _) => 1,
                };
                let (_, h, c, s, m, sp, e) = PRESETS[i];
                (p.history, p.civs, p.savagery, p.minerals, p.spacing, p.epics) = (h, c, s, m, sp, e);
            }
            Row::Seed => {}
            Row::Size => {
                let k = SIZE_ORDER.iter().position(|&s| s == p.size).unwrap_or(1);
                let k = if p.width > 0 {
                    // From a custom width, step to the nearest preset that way.
                    let w = p.width as i32;
                    let dims = |s: u8| WorldParams { size: s, width: 0, ..*p }.dims().0;
                    let pos = SIZE_ORDER.iter().position(|&s| if right { dims(s) > w } else { dims(s) >= w }).unwrap_or(SIZE_ORDER.len());
                    if right {
                        pos.min(SIZE_ORDER.len() - 1)
                    } else {
                        pos.saturating_sub(1)
                    }
                } else if right {
                    (k + 1).min(SIZE_ORDER.len() - 1)
                } else {
                    k.saturating_sub(1)
                };
                p.size = SIZE_ORDER[k];
                p.width = 0;
            }
            Row::History => {
                let i = HISTORY_CHOICES.iter().position(|&h| h >= p.history).unwrap_or(3);
                let i = if right { (i + 1).min(HISTORY_CHOICES.len() - 1) } else { i.saturating_sub(1) };
                p.history = HISTORY_CHOICES[i];
            }
            Row::Civs => p.civs = lvl(p.civs, 4),
            Row::Spacing => p.spacing = lvl(p.spacing, 2),
            Row::Epics => p.epics = lvl(p.epics, 2),
            Row::Savagery => p.savagery = lvl(p.savagery, 2),
            Row::Minerals => p.minerals = lvl(p.minerals, 2),
            Row::Ocean => p.ocean = lvl(p.ocean, 4),
            Row::Mountains => p.mountains = lvl(p.mountains, 4),
            Row::Rain => p.rain = lvl(p.rain, 4),
            Row::Heat => p.heat = lvl(p.heat, 4),
            Row::Forest => p.forest = lvl(p.forest, 4),
            Row::Generate => {}
        }
    }
}

/// Widen or narrow the world by 16 cells (a custom size), within 64..512.
pub fn nudge_width(p: &mut WorldParams, wider: bool) {
    let w = p.dims().0;
    let w = if wider { w + 16 } else { w - 16 };
    p.width = w.clamp(64, 512) as u16;
}

/// Every setting back to the usual, keeping the seed.
pub fn defaults(p: &mut WorldParams) {
    *p = WorldParams { seed: p.seed, ..WorldParams::default() };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_steps_both_ways_and_reads_back() {
        let mut p = WorldParams::default();
        for (row, _) in ROWS {
            let before = row.value(&p);
            row.step(&mut p, true);
            row.step(&mut p, false);
            if !matches!(row, Row::Preset | Row::Size | Row::History) {
                assert_eq!(row.value(&p), before, "{:?} doesn't step back", row);
            }
            assert!(!row.label().is_empty());
        }
        // Sizes run Tiny to Huge, and a custom width steps back onto them.
        let mut p = WorldParams { size: 4, ..WorldParams::default() };
        let mut seen = vec![p.dims().0];
        for _ in 0..6 {
            Row::Size.step(&mut p, true);
            seen.push(p.dims().0);
        }
        assert_eq!(seen, vec![96, 128, 192, 288, 384, 384, 384]);
        nudge_width(&mut p, false);
        assert_eq!(p.dims(), (368, 153));
        assert!(Row::Size.value(&p).starts_with("Custom"));
        Row::Size.step(&mut p, false);
        assert_eq!((p.width, p.dims().0), (0, 288));
        // Presets keep their names; anything else is Custom.
        let mut p = WorldParams::default();
        assert_eq!(preset_name(&p), "Standard");
        p.civs = 4;
        assert_eq!(preset_name(&p), "Custom");
        defaults(&mut p);
        assert_eq!(preset_name(&p), "Standard");
    }

    /// The same settings and seed make the same world, the settings are
    /// kept with it, and the land knobs move the land the way they say.
    #[test]
    fn settings_shape_the_land_and_are_kept() {
        use crate::overworld::{Biome, WorldGen};
        let base = WorldParams { seed: 21, history: 30, ..WorldParams::default() };
        let make = |p: WorldParams| WorldGen::new(p).finish();
        let share = |ow: &crate::overworld::Overworld, f: &dyn Fn(Biome) -> bool| ow.cells.iter().filter(|c| f(c.biome)).count() as f64 / ow.cells.len() as f64;
        let a = make(base);
        assert_eq!(a.made_with, Some(base));
        let json = serde_json::to_string(&a).unwrap();
        let back: crate::overworld::Overworld = serde_json::from_str(&json).unwrap();
        assert_eq!(back.made_with, Some(base));
        assert!(json == serde_json::to_string(&make(base)).unwrap(), "same settings, different world");
        let sea = |ow: &crate::overworld::Overworld| share(ow, &|b| b == Biome::Ocean);
        let wet = make(WorldParams { ocean: 4, ..base });
        let dry = make(WorldParams { ocean: 0, ..base });
        assert!(sea(&wet) > sea(&a) + 0.05 && sea(&dry) < sea(&a) - 0.05, "ocean {:.2} {:.2} {:.2}", sea(&dry), sea(&a), sea(&wet));
        let peaks = |ow: &crate::overworld::Overworld| share(ow, &|b| matches!(b, Biome::Mountain | Biome::Hills | Biome::Glacier | Biome::Badlands));
        assert!(peaks(&make(WorldParams { mountains: 4, ..base })) > peaks(&a) + 0.04);
        let desert = |ow: &crate::overworld::Overworld| share(ow, &|b| matches!(b, Biome::Desert | Biome::Badlands | Biome::Shrubland));
        assert!(desert(&make(WorldParams { rain: 0, ..base })) > desert(&a) + 0.04);
        let cold = |ow: &crate::overworld::Overworld| share(ow, &|b| matches!(b, Biome::Tundra | Biome::Glacier | Biome::Taiga));
        assert!(cold(&make(WorldParams { heat: 0, ..base })) > cold(&a) + 0.04);
        assert!(cold(&make(WorldParams { heat: 4, ..base })) < cold(&a));
        let trees = |ow: &crate::overworld::Overworld| share(ow, &|b| matches!(b, Biome::Forest | Biome::Jungle | Biome::Taiga));
        assert!(trees(&make(WorldParams { forest: 4, ..base })) > trees(&a) + 0.03);
        // A custom size makes a world that size.
        let c = make(WorldParams { width: 160, ..base });
        assert_eq!((c.w, c.h), (160, 66));
    }
}
