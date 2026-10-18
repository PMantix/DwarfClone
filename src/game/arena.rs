//! The arena (WISHLIST G01): a battle you set up and watch. Two sides, any
//! creatures, any kit, on a field of your choosing, fought with the same
//! blows, blood and severed parts as a siege (`combat::strike`,
//! `aftermath`, `gore`). Nobody eats, sleeps or works here: the fighters
//! live in a little `Game` of their own that only the arena's loop moves
//! (`fight.rs`). A matchup can be saved to `saves/` and rerun many times
//! for win rates, and the overseer war-games a coming siege in it
//! (`wargame.rs`).

use super::*;
use crate::creature::Species;
use std::path::{Path, PathBuf};

mod fields;
mod fight;
pub use fight::{Battle, Tally, SIDE_COLORS};

/// Who fights on a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kin {
    Dwarf,
    Beast(Species),
}

/// Everyone the arena can field, people first, then beasts by size.
pub const KINS: [Kin; 30] = [
    Kin::Dwarf,
    Kin::Beast(Species::Goblin),
    Kin::Beast(Species::Human),
    Kin::Beast(Species::Elf),
    Kin::Beast(Species::Raider),
    Kin::Beast(Species::Merchant),
    Kin::Beast(Species::Visitor),
    Kin::Beast(Species::Skulker),
    Kin::Beast(Species::Undead),
    Kin::Beast(Species::Husk),
    Kin::Beast(Species::Troglodyte),
    Kin::Beast(Species::Megabeast),
    Kin::Beast(Species::CaveHorror),
    Kin::Beast(Species::GiantSpider),
    Kin::Beast(Species::PolarBear),
    Kin::Beast(Species::Bear),
    Kin::Beast(Species::Alligator),
    Kin::Beast(Species::Lion),
    Kin::Beast(Species::Jaguar),
    Kin::Beast(Species::Wolf),
    Kin::Beast(Species::Dog),
    Kin::Beast(Species::Scorpion),
    Kin::Beast(Species::Viper),
    Kin::Beast(Species::Cat),
    Kin::Beast(Species::Elk),
    Kin::Beast(Species::Camel),
    Kin::Beast(Species::Deer),
    Kin::Beast(Species::Goat),
    Kin::Beast(Species::Ibex),
    Kin::Beast(Species::Antelope),
];

impl Kin {
    pub fn name(self) -> &'static str {
        match self {
            Kin::Dwarf => "dwarf",
            Kin::Beast(s) => s.info().name,
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Kin::Dwarf => "dwarves",
            Kin::Beast(s) => s.info().plural,
        }
    }

    /// People hold weapons and wear armour; beasts fight with what they have.
    pub fn armed(self) -> bool {
        match self {
            Kin::Dwarf => true,
            Kin::Beast(s) => s.person() || matches!(s, Species::Skulker | Species::Undead | Species::Troglodyte),
        }
    }

    pub fn glyph(self) -> char {
        match self {
            Kin::Dwarf => '☻',
            Kin::Beast(s) => s.info().glyph,
        }
    }

    fn index(self) -> usize {
        KINS.iter().position(|&k| k == self).unwrap_or(0)
    }
}

/// What a side can hold (None: bare hands, or teeth and claws).
pub const WEAPONS: [Option<ItemKind>; 7] = [None, Some(ItemKind::Axe), Some(ItemKind::Sword), Some(ItemKind::Spear), Some(ItemKind::Hammer), Some(ItemKind::Pick), Some(ItemKind::Crossbow)];
pub fn weapon_name(k: ItemKind) -> &'static str {
    match k {
        ItemKind::Axe => "axe",
        ItemKind::Sword => "sword",
        ItemKind::Spear => "spear",
        ItemKind::Hammer => "hammer",
        ItemKind::Pick => "pick",
        ItemKind::Crossbow => "crossbow",
        _ => "weapon",
    }
}

/// What weapons and bolts are made of.
pub const WEAPON_MATS: [Mat; 6] = [Mat::Wood, Mat::Bone, Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel];
/// What armour is made of.
pub const ARMOUR_MATS: [Mat; 6] = [Mat::Leather, Mat::Bone, Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel];
/// Sets of armour: (name, helm, breastplate, shield).
pub const KITS: [(&str, bool, bool, bool); 6] = [
    ("none", false, false, false),
    ("helm", true, false, false),
    ("shield", false, false, true),
    ("helm and shield", true, false, true),
    ("helm and mail", true, true, false),
    ("helm, mail and shield", true, true, true),
];

/// How a dwarf's fighting skill reads (`dwarf::skill_rank`), or a beast's
/// or a people's training.
pub const TRAINING: [&str; 6] = ["wild", "trained", "drilled", "veteran", "elite", "legendary"];

/// One side of a matchup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SideSpec {
    pub kin: Kin,
    pub count: u32,
    /// A dwarf's fighting skill rank (0..=10), or anyone else's training
    /// (0..=5), which adds to their skill.
    pub skill: u32,
    pub weapon: Option<ItemKind>,
    pub weapon_mat: Mat,
    /// Index into `KITS`.
    pub kit: usize,
    pub armour_mat: Mat,
    /// Bolts each, for those with crossbows.
    #[serde(default)]
    pub bolts: u32,
}

impl SideSpec {
    pub fn new(kin: Kin, count: u32) -> SideSpec {
        SideSpec { kin, count, skill: 0, weapon: None, weapon_mat: Mat::Iron, kit: 0, armour_mat: Mat::Iron, bolts: 20 }
    }

    pub fn armed(mut self, weapon: ItemKind, mat: Mat) -> SideSpec {
        self.weapon = Some(weapon);
        self.weapon_mat = mat;
        self
    }

    pub fn kitted(mut self, kit: usize, mat: Mat) -> SideSpec {
        self.kit = kit;
        self.armour_mat = mat;
        self
    }

    pub fn skilled(mut self, skill: u32) -> SideSpec {
        self.skill = skill;
        self
    }

    /// The most a side's skill can be set to.
    pub fn max_skill(&self) -> u32 {
        if self.kin == Kin::Dwarf {
            10
        } else {
            TRAINING.len() as u32 - 1
        }
    }

    pub fn skill_name(&self) -> String {
        if self.kin == Kin::Dwarf {
            crate::dwarf::skill_rank(self.skill * 400).to_string()
        } else {
            TRAINING[self.skill.min(5) as usize].to_string()
        }
    }

    /// "10 dwarves with iron spears, helm and mail"
    pub fn describe(&self) -> String {
        let who = if self.count == 1 { format!("a {}", self.kin.name()) } else { format!("{} {}", self.count, self.kin.plural()) };
        if !self.kin.armed() {
            return who;
        }
        let mut s = who;
        if let Some(w) = self.weapon {
            let wn = weapon_name(w);
            s.push_str(&format!(" with {} {}{}", self.weapon_mat.name().to_lowercase(), wn, if self.count == 1 { "" } else { "s" }));
        }
        if self.kit > 0 {
            s.push_str(&format!(", {} {}", self.armour_mat.name().to_lowercase(), KITS[self.kit].0));
        }
        s
    }

    /// The fighters this side sends, one by one.
    pub fn recruits(&self) -> Vec<Recruit> {
        let armed = self.kin.armed();
        let (_, helm, mail, shield) = KITS[self.kit.min(KITS.len() - 1)];
        let mut armour = Vec::new();
        if armed {
            for (on, k) in [(helm, ItemKind::Helm), (mail, ItemKind::Breastplate), (shield, ItemKind::Shield)] {
                if on {
                    armour.push((k, self.armour_mat));
                }
            }
        }
        let weapon = if armed { self.weapon.map(|w| (w, self.weapon_mat)) } else { None };
        let bolts = if weapon.map_or(false, |(w, _)| w == ItemKind::Crossbow) { self.bolts } else { 0 };
        (0..self.count)
            .map(|_| Recruit {
                kin: self.kin,
                name: None,
                fight_xp: if self.kin == Kin::Dwarf { self.skill * 400 } else { 0 },
                training: if self.kin == Kin::Dwarf { 0 } else { self.skill },
                weapon,
                armour: armour.clone(),
                bolts,
                bolt_mat: self.weapon_mat,
                big: false,
            })
            .collect()
    }
}

/// One fighter as sent into the arena.
#[derive(Clone, Debug, PartialEq)]
pub struct Recruit {
    pub kin: Kin,
    /// Their own name (soldiers of the fort in a war-game); else one is made up.
    pub name: Option<String>,
    pub fight_xp: u32,
    /// Added to a creature's skill.
    pub training: u32,
    pub weapon: Option<(ItemKind, Mat)>,
    pub armour: Vec<(ItemKind, Mat)>,
    pub bolts: u32,
    pub bolt_mat: Mat,
    /// A leader: a third bigger.
    pub big: bool,
}

/// Where they fight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field {
    /// Grass inside a palisade, a few trees.
    Open,
    /// Two halls joined by a long corridor with a door halfway (beasts
    /// can't open doors: they have to batter it down).
    Corridor,
    /// A winding cave with rock pillars.
    Cavern,
    /// A chasm with a narrow bridge over it: a heavy blow can send someone
    /// over the edge.
    Bridge,
    /// A paved yard inside walls, with pillars and low walls for cover.
    Yard,
}

impl Field {
    pub const ALL: [Field; 5] = [Field::Open, Field::Corridor, Field::Cavern, Field::Bridge, Field::Yard];

    pub fn name(self) -> &'static str {
        match self {
            Field::Open => "open ground",
            Field::Corridor => "a corridor with a door",
            Field::Cavern => "a cavern",
            Field::Bridge => "a bridge over a chasm",
            Field::Yard => "a walled yard",
        }
    }

    fn index(self) -> usize {
        Field::ALL.iter().position(|&f| f == self).unwrap_or(0)
    }
}

/// Two sides and a field: what the setup screen builds and `saves/` keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matchup {
    pub name: String,
    pub sides: [SideSpec; 2],
    pub field: Field,
    /// The first bout's seed; reruns count on from it.
    #[serde(default)]
    pub seed: u64,
}

impl Default for Matchup {
    /// The WISHLIST's own: ten dwarves with spears against six goblins.
    fn default() -> Matchup {
        presets().remove(0)
    }
}

/// Matchups worth a look, each with a story in it.
pub fn presets() -> Vec<Matchup> {
    let d = |n| SideSpec::new(Kin::Dwarf, n);
    let b = |s, n| SideSpec::new(Kin::Beast(s), n);
    let m = |name: &str, a: SideSpec, z: SideSpec, field| Matchup { name: name.to_string(), sides: [a, z], field, seed: 1 };
    vec![
        m("Ten spears against six goblins", d(10).armed(ItemKind::Spear, Mat::Iron).kitted(1, Mat::Copper).skilled(1), b(Species::Goblin, 6).armed(ItemKind::Sword, Mat::Bronze).kitted(1, Mat::Bronze), Field::Open),
        m("The duel", d(1).armed(ItemKind::Axe, Mat::Iron).kitted(5, Mat::Iron), b(Species::Goblin, 1).armed(ItemKind::Axe, Mat::Iron).kitted(5, Mat::Iron), Field::Open),
        m("Hold the bridge", d(3).armed(ItemKind::Axe, Mat::Steel).kitted(5, Mat::Steel).skilled(5), b(Species::Goblin, 12).armed(ItemKind::Spear, Mat::Copper).kitted(1, Mat::Copper), Field::Bridge),
        m("A bear at the door", d(4).armed(ItemKind::Axe, Mat::Copper).skilled(1), b(Species::Bear, 1), Field::Corridor),
        m("The wolf pack", d(5).armed(ItemKind::Crossbow, Mat::Iron).kitted(1, Mat::Leather).skilled(2), b(Species::Wolf, 8), Field::Cavern),
        m("A titan's due", d(12).armed(ItemKind::Hammer, Mat::Iron).kitted(5, Mat::Iron).skilled(3), b(Species::Megabeast, 1), Field::Yard),
        m("The dead walk", d(6).armed(ItemKind::Sword, Mat::Bronze).kitted(4, Mat::Bronze).skilled(2), b(Species::Undead, 15), Field::Cavern),
        m("Marksdwarves and the horror", d(8).armed(ItemKind::Crossbow, Mat::Steel).kitted(1, Mat::Iron).skilled(3), b(Species::CaveHorror, 1), Field::Yard),
        m("Old rivals", d(6).armed(ItemKind::Axe, Mat::Bronze).kitted(3, Mat::Bronze).skilled(2), b(Species::Raider, 6).armed(ItemKind::Hammer, Mat::Iron).kitted(1, Mat::Iron), Field::Corridor),
        m("Elves in the yard", b(Species::Elf, 8).armed(ItemKind::Spear, Mat::Wood).kitted(2, Mat::Wood), b(Species::Human, 6).armed(ItemKind::Sword, Mat::Iron).kitted(5, Mat::Iron), Field::Yard),
    ]
}

/// A matchup made up out of thin air, the two sides roughly a match for
/// each other (counts set by how hard each kind hits).
pub fn surprise(seed: u64) -> Matchup {
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(17));
    let side = |rng: &mut Rng| {
        let kin = KINS[rng.range(0, KINS.len() as i32) as usize];
        let mut s = SideSpec::new(kin, 1);
        if kin.armed() {
            s.weapon = WEAPONS[rng.range(1, WEAPONS.len() as i32) as usize];
            s.weapon_mat = WEAPON_MATS[rng.range(2, WEAPON_MATS.len() as i32) as usize];
            s.kit = rng.range(0, KITS.len() as i32) as usize;
            s.armour_mat = ARMOUR_MATS[rng.range(0, ARMOUR_MATS.len() as i32) as usize];
        }
        s.skill = rng.range(0, s.max_skill() as i32 / 2 + 1) as u32;
        s
    };
    let (mut a, mut b) = (side(&mut rng), side(&mut rng));
    // Weight of a fighter: blood times bite, near enough.
    let weight = |s: &SideSpec| match s.kin {
        Kin::Dwarf => 1.0 + 0.4 * s.weapon.is_some() as i32 as f64 + 0.1 * s.kit as f64,
        Kin::Beast(sp) => {
            let i = sp.info();
            (i.scale as f64 * i.damage as f64 / 3.0).max(0.2) * (1.0 + 0.3 * s.weapon.is_some() as i32 as f64)
        }
    };
    let total = 4.0 + rng.range(0, 8) as f64;
    a.count = ((total / weight(&a)).round() as u32).clamp(1, 30);
    b.count = ((total / weight(&b)).round() as u32).clamp(1, 30);
    let field = Field::ALL[rng.range(0, Field::ALL.len() as i32) as usize];
    let name = format!("{} against {}", super::combat::cap(a.kin.plural()), b.kin.plural());
    Matchup { name, sides: [a, b], field, seed: seed % 10_000 + 1 }
}

impl Matchup {
    /// What the bout is, in a line.
    pub fn describe(&self) -> String {
        format!("{} against {}, on {}", self.sides[0].describe(), self.sides[1].describe(), self.field.name())
    }

    /// Step a setting on (`dir` 1) or back (-1). `row` is one of the setup
    /// screen's rows (`SETUP_ROWS`); `side` 2 is the field.
    pub fn adjust(&mut self, side: usize, row: usize, dir: i32) {
        let cycle = |i: usize, n: usize| ((i as i32 + dir).rem_euclid(n as i32)) as usize;
        if side >= 2 || row == ROW_FIELD {
            self.field = Field::ALL[cycle(self.field.index(), Field::ALL.len())];
            return;
        }
        let s = &mut self.sides[side];
        match row {
            ROW_KIN => {
                s.kin = KINS[cycle(s.kin.index(), KINS.len())];
                s.skill = s.skill.min(s.max_skill());
            }
            ROW_COUNT => {
                let step = if s.count >= 10 && dir > 0 || s.count > 10 && dir < 0 { 5 } else { 1 };
                s.count = (s.count as i32 + dir * step).clamp(1, 40) as u32;
            }
            ROW_SKILL => s.skill = (s.skill as i32 + dir).clamp(0, s.max_skill() as i32) as u32,
            ROW_WEAPON => {
                let i = WEAPONS.iter().position(|&w| w == s.weapon).unwrap_or(0);
                s.weapon = WEAPONS[cycle(i, WEAPONS.len())];
            }
            ROW_WEAPON_MAT => {
                let i = WEAPON_MATS.iter().position(|&m| m == s.weapon_mat).unwrap_or(4);
                s.weapon_mat = WEAPON_MATS[cycle(i, WEAPON_MATS.len())];
            }
            ROW_KIT => s.kit = cycle(s.kit, KITS.len()),
            ROW_ARMOUR_MAT => {
                let i = ARMOUR_MATS.iter().position(|&m| m == s.armour_mat).unwrap_or(4);
                s.armour_mat = ARMOUR_MATS[cycle(i, ARMOUR_MATS.len())];
            }
            ROW_BOLTS => s.bolts = (s.bolts as i32 + dir * 5).clamp(0, 60) as u32,
            _ => {}
        }
    }

    /// One row of the setup screen for a side: (label, value, in use).
    pub fn row(&self, side: usize, row: usize) -> (&'static str, String, bool) {
        let s = &self.sides[side.min(1)];
        let armed = s.kin.armed();
        match row {
            ROW_KIN => ("Who", s.kin.plural().to_string(), true),
            ROW_COUNT => ("How many", s.count.to_string(), true),
            ROW_SKILL => (if s.kin == Kin::Dwarf { "Fighting" } else { "Training" }, s.skill_name(), true),
            ROW_WEAPON => ("Weapon", if armed { s.weapon.map_or("bare hands".to_string(), |w| weapon_name(w).to_string()) } else { s.kin.name_of_attack() }, armed),
            ROW_WEAPON_MAT => ("  made of", s.weapon_mat.name().to_lowercase(), armed && s.weapon.is_some()),
            ROW_KIT => ("Armour", KITS[s.kit.min(KITS.len() - 1)].0.to_string(), armed),
            ROW_ARMOUR_MAT => ("  made of", s.armour_mat.name().to_lowercase(), armed && s.kit > 0),
            ROW_BOLTS => ("Bolts each", s.bolts.to_string(), armed && s.weapon == Some(ItemKind::Crossbow)),
            _ => ("", String::new(), false),
        }
    }

    /// A file name for it in `saves/`.
    pub fn file_name(&self) -> String {
        let slug: String = self.name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        format!("{}{}", if slug.is_empty() { "matchup".to_string() } else { slug }, SUFFIX)
    }
}

impl Kin {
    /// What a beast fights with ("bites"): shown where a weapon would be.
    pub fn name_of_attack(self) -> String {
        match self {
            Kin::Dwarf => "fists".into(),
            Kin::Beast(s) => format!("{} ({})", if s.info().edged { "claws and teeth" } else { "brute force" }, s.info().verb),
        }
    }
}

/// The setup screen's rows for each side, then the field.
pub const ROW_KIN: usize = 0;
pub const ROW_COUNT: usize = 1;
pub const ROW_SKILL: usize = 2;
pub const ROW_WEAPON: usize = 3;
pub const ROW_WEAPON_MAT: usize = 4;
pub const ROW_KIT: usize = 5;
pub const ROW_ARMOUR_MAT: usize = 6;
pub const ROW_BOLTS: usize = 7;
pub const ROW_FIELD: usize = 8;
pub const SETUP_ROWS: usize = 9;

/// Saved matchups end in this.
pub const SUFFIX: &str = ".arena.json";

/// Keep a matchup in `dir` (as `<name>.arena.json`); the path it went to.
pub fn save_matchup(dir: &Path, m: &Matchup) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(m.file_name());
    let data = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn load_matchup(path: &Path) -> Result<Matchup, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| format!("Not a matchup: {}", e))
}

/// Every matchup saved in `dir`, by name.
pub fn saved_matchups(dir: &Path) -> Vec<Matchup> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<Matchup> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(SUFFIX))
        .filter_map(|e| load_matchup(&e.path()).ok())
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matchup_is_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("dwarfclone-arena-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut m = Matchup::default();
        m.adjust(1, ROW_COUNT, 1);
        m.adjust(0, ROW_WEAPON_MAT, 1);
        m.adjust(2, ROW_FIELD, 1);
        m.name = "Spears & goblins: the rematch!".into();
        let path = save_matchup(&dir, &m).unwrap();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "spears-goblins-the-rematch.arena.json");
        assert_eq!(load_matchup(&path).unwrap(), m);
        // A fortress's save beside it isn't taken for one.
        std::fs::write(dir.join("Goldfist.json"), "{}").unwrap();
        let all = saved_matchups(&dir);
        assert_eq!(all, vec![m.clone()]);
        assert_eq!(all[0].sides[1].count, 7);
        assert_eq!(all[0].sides[0].weapon_mat, Mat::Steel);
        assert_eq!(all[0].field, Field::Corridor);
        // Older or hand-written files without the optional fields still load.
        let bare = r#"{"name":"x","sides":[{"kin":"Dwarf","count":2,"skill":0,"weapon":null,"weapon_mat":"Iron","kit":0,"armour_mat":"Iron"},{"kin":{"Beast":"Bear"},"count":1,"skill":0,"weapon":null,"weapon_mat":"Iron","kit":0,"armour_mat":"Iron"}],"field":"Bridge"}"#;
        let m: Matchup = serde_json::from_str(bare).unwrap();
        assert_eq!(m.sides[1].kin, Kin::Beast(Species::Bear));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_setting_cycles_and_describes() {
        let mut m = Matchup::default();
        for side in 0..2 {
            for row in 0..ROW_FIELD {
                for _ in 0..40 {
                    m.adjust(side, row, 1);
                    let (label, value, _) = m.row(side, row);
                    assert!(!label.is_empty() && !value.is_empty());
                }
                m.adjust(side, row, -1);
            }
        }
        assert!(m.sides.iter().all(|s| s.count >= 1 && s.count <= 40 && s.skill <= s.max_skill()));
        assert!(!m.describe().is_empty());
        let d = Matchup::default();
        assert_eq!(d.sides[0].count, 10);
        assert_eq!(d.sides[1].kin, Kin::Beast(Species::Goblin));
        assert_eq!(d.sides[1].count, 6);
        assert!(d.describe().starts_with("10 dwarves with iron spears"), "{}", d.describe());
        for seed in 0..30 {
            let s = surprise(seed);
            assert!(s.sides.iter().all(|x| x.count >= 1));
        }
    }
}
