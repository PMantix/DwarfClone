//! The overseer war-games a coming siege in the arena (G01 put to use).
//! When the scouts report an army, it sends its soldiers, kitted as they
//! are, against that many of the foe, armed as such armies come, a dozen
//! times over on open ground. The odds are told (an announcement and a
//! card) and go in the year's chronicle; grim odds send everyone inside
//! well before the army comes (`wargame_lead`, read by the alert check).
//! When the siege ends, the war-game is set against what happened.

use super::arena::{Battle, Field, Kin, Recruit};
use super::overseer::Overseer;
use super::*;
use crate::creature::Species;

/// Bouts in a war-game.
pub const BOUTS: u32 = 12;

/// The last war-game: what it was for and how it went.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WarGame {
    /// When the army it was for was due (the scouts' `incoming`).
    pub due: u64,
    pub wins: u32,
    pub bouts: u32,
    pub soldiers: u32,
    pub foes: u32,
    /// "goblins"
    pub foe: String,
    /// Set against the real siege already.
    #[serde(default)]
    pub judged: bool,
}

impl WarGame {
    /// Fewer than one bout in three won.
    pub fn grim(&self) -> bool {
        self.bouts > 0 && self.wins * 3 < self.bouts
    }

    /// Two in three or better.
    pub fn confident(&self) -> bool {
        self.bouts > 0 && self.wins * 3 >= self.bouts * 2
    }
}

/// What the `i`th of an army brings, as `lay_siege_of` arms them (the
/// usual spread of it rather than a roll): species, weapon and armour.
pub fn army_recruit(race: Race, i: u32) -> Recruit {
    let (species, mats, weapons, armour): (Species, &[Mat], &[ItemKind], [u32; 3]) = match race {
        Race::Goblin => (Species::Goblin, &[Mat::Copper, Mat::Copper, Mat::Bronze, Mat::Iron], &[ItemKind::Spear, ItemKind::Sword, ItemKind::Axe, ItemKind::Hammer], [2, 3, 4]),
        Race::Human => (Species::Human, &[Mat::Bronze, Mat::Iron, Mat::Iron], &[ItemKind::Sword, ItemKind::Spear, ItemKind::Hammer], [1, 2, 2]),
        Race::Elf => (Species::Elf, &[Mat::Wood], &[ItemKind::Spear, ItemKind::Sword], [3, 0, 2]),
        Race::Dwarf => (Species::Raider, &[Mat::Bronze, Mat::Iron, Mat::Iron], &[ItemKind::Axe, ItemKind::Hammer], [1, 1, 2]),
    };
    let i = i as usize;
    let mat = mats[i % mats.len()];
    let leads = i == 0;
    // One in `every` wears each piece (helm, mail, shield); the leader all of them.
    let wears = |every: u32| every > 0 && (leads || i % every as usize == 1);
    let mut kit = Vec::new();
    for (k, every) in [(ItemKind::Helm, armour[0]), (ItemKind::Breastplate, armour[1]), (ItemKind::Shield, armour[2])] {
        if wears(every) {
            kit.push((k, mat));
        }
    }
    Recruit { kin: Kin::Beast(species), name: None, fight_xp: 0, training: 0, weapon: Some((weapons[(i / 2) % weapons.len()], mat)), armour: kit, bolts: 0, bolt_mat: mat, big: leads }
}

impl Game {
    /// A soldier as the arena sends them: their skill and their kit.
    fn recruit_of(&self, d: &Dwarf) -> Recruit {
        let item = |id: &u32| self.items.get(id).map(|it| (it.kind, it.mat));
        let weapon = d.held.as_ref().and_then(item).filter(|(k, _)| k.weapon_damage().is_some());
        let armour: Vec<(ItemKind, Mat)> = d.armor.iter().filter_map(item).filter(|(k, _)| k.armor_slot().is_some()).collect();
        Recruit { kin: Kin::Dwarf, name: Some(d.called()), fight_xp: d.fight_xp, training: 0, weapon, armour, bolts: d.ammo, bolt_mat: d.ammo_mat.unwrap_or(Mat::Wood), big: false }
    }

    /// From the alert hooks: an army has been seen and not yet war-gamed.
    pub(super) fn ov_wargame(&mut self, _o: &mut Overseer) {
        let Some((due, foe, n, _)) = self.incoming.clone() else { return };
        if self.wargame.as_ref().map_or(false, |w| w.due == due) {
            return;
        }
        let soldiers: Vec<Recruit> = self.dwarves.iter().filter(|d| d.alive && d.soldier && !d.is_child()).map(|d| self.recruit_of(d)).collect();
        let foes: Vec<Recruit> = (0..n).map(|i| army_recruit(foe.race, i)).collect();
        let plural = match foes.first().map(|r| r.kin) {
            Some(k) => k.plural().to_string(),
            None => "foes".to_string(),
        };
        let mut wg = WarGame { due, wins: 0, bouts: 0, soldiers: soldiers.len() as u32, foes: n, foe: plural.clone(), judged: false };
        if soldiers.is_empty() {
            self.wargame = Some(wg);
            return;
        }
        for k in 0..BOUTS {
            let mut b = Battle::with_recruits("War-game", Field::Open, [soldiers.clone(), foes.clone()], due.wrapping_mul(31).wrapping_add(k as u64));
            if b.run().winner == Some(0) {
                wg.wins += 1;
            }
            wg.bouts += 1;
        }
        let verdict = if wg.confident() {
            "The soldiers are confident."
        } else if wg.grim() {
            "It looks grim: everyone is to be inside long before they come."
        } else {
            "It will be close."
        };
        let s = if wg.soldiers == 1 { "soldier" } else { "soldiers" };
        let text = format!("The overseer war-games the coming fight in the arena: our {} {} against {} {} win {} bouts in {}. {}", wg.soldiers, s, n, plural, wg.wins, wg.bouts, verdict);
        self.msg(&text, (235, 200, 120));
        self.spotlight("The war-game", &text, None, (235, 200, 120));
        self.year_log.wargame = Some(format!("Before the {} came, the overseer war-gamed the fight in the arena, and our soldiers won {} bouts in {}.", plural, wg.wins, wg.bouts));
        self.count("arena.wargames", 1);
        if wg.grim() {
            self.count("arena.wargame_grim", 1);
        }
        self.wargame = Some(wg);
    }

    /// How long before an army is due the alert goes up: a day when the
    /// war-game looked grim, else the usual few hours.
    pub(super) fn wargame_lead(&self) -> u64 {
        let due = self.incoming.as_ref().map(|i| i.0);
        match &self.wargame {
            Some(w) if Some(w.due) == due && w.grim() => self.knob("wargame.grim_lead", DAY as f64) as u64,
            _ => 400 + self.march_lead(),
        }
    }

    /// The siege is over: was the war-game right?
    pub(super) fn judge_wargame(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i) else { return };
        let won = s.outcome == "routed";
        let Some(w) = self.wargame.as_mut().filter(|w| !w.judged && w.bouts > 0 && w.due <= s.tick + DAY) else { return };
        w.judged = true;
        let (wins, bouts) = (w.wins, w.bouts);
        let foreseen = (wins * 2 >= bouts) == won;
        let text = match (foreseen, won) {
            (true, true) => format!("The war-game gave us {} bouts in {}, and so it went: the attackers were broken.", wins, bouts),
            (true, false) => format!("The war-game gave us only {} bouts in {}, and the arena was right.", wins, bouts),
            (false, true) => format!("The war-game gave us only {} bouts in {}, but the soldiers did better than the arena.", wins, bouts),
            (false, false) => format!("The war-game gave us {} bouts in {}, but the real fight went worse.", wins, bouts),
        };
        self.msg(&text, (235, 200, 120));
        if foreseen {
            self.count("arena.wargame_right", 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_war_games_a_coming_siege_and_tells_the_odds() {
        let mut g = trial_game(5003, Style::Steady, 0);
        // Four soldiers in iron.
        let e = g.embark;
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).take(4).map(|d| d.id).collect();
        for &id in &ids {
            let kit: Vec<u32> = [ItemKind::Axe, ItemKind::Helm, ItemKind::Breastplate, ItemKind::Shield].iter().map(|&k| g.spawn_item(k, Mat::Iron, e)).collect();
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.soldier = true;
            d.fight_xp = 800;
            d.held = Some(kit[0]);
            d.armor = kit[1..].to_vec();
        }
        let foe = Neighbor { civ: 0, race: Race::Goblin, dist: 5 };
        g.incoming = Some((g.tick + DAY, foe.clone(), 3, "Snaga".into()));
        let mut o = g.overseer.take().unwrap();
        g.ov_wargame(&mut o);
        let w = g.wargame.clone().expect("no war-game was fought");
        assert_eq!((w.soldiers, w.foes, w.bouts), (4, 3, BOUTS));
        assert!(w.confident(), "four in iron should beat three goblins: {} in {}", w.wins, w.bouts);
        assert!(g.log.iter().any(|l| l.text.contains("war-games the coming fight")));
        assert!(g.spotlights.iter().any(|s| s.title == "The war-game"));
        assert!(g.year_log.wargame.is_some());
        assert_eq!(g.wargame_lead(), 400);
        // Once only for each army.
        let before = g.log.len();
        g.ov_wargame(&mut o);
        assert_eq!(g.log.len(), before);
        // A bigger army against one recruit with bare hands: grim, and the
        // alert goes up a day ahead.
        for d in g.dwarves.iter_mut() {
            d.soldier = false;
        }
        let d = g.dwarves.iter_mut().find(|d| d.id == ids[0]).unwrap();
        d.soldier = true;
        d.held = None;
        d.armor.clear();
        d.fight_xp = 0;
        g.incoming = Some((g.tick + DAY + 5, foe, 12, "Snaga".into()));
        g.ov_wargame(&mut o);
        let w = g.wargame.clone().unwrap();
        assert!(w.grim(), "one bare-handed dwarf against twelve: {} in {}", w.wins, w.bouts);
        assert_eq!(g.wargame_lead(), DAY);
        assert!(g.log.last().unwrap().text.contains("grim"));
        // The siege comes and goes: the war-game is set against it.
        g.annals.sieges.push(annals::SiegeNote { tick: g.tick + DAY + 5, foe: "goblins".into(), size: 12, outcome: "withdrew".into(), ..Default::default() });
        let i = g.annals.sieges.len() - 1;
        g.judge_wargame(i);
        assert!(g.log.last().unwrap().text.contains("the arena was right"), "{}", g.log.last().unwrap().text);
        assert_eq!(g.annals.counters.get("arena.wargame_right"), Some(&1));
        g.overseer = Some(o);
    }

    #[test]
    fn an_army_is_armed_as_sieges_arm_them() {
        let lead = army_recruit(Race::Goblin, 0);
        assert!(lead.big && lead.armour.len() == 3);
        let all: Vec<Recruit> = (0..12).map(|i| army_recruit(Race::Goblin, i)).collect();
        assert!(all.iter().all(|r| r.weapon.is_some() && r.kin == Kin::Beast(Species::Goblin)));
        let helms = all.iter().filter(|r| r.armour.iter().any(|a| a.0 == ItemKind::Helm)).count();
        assert!((4..=8).contains(&helms), "{} helms in 12", helms);
        assert!(army_recruit(Race::Elf, 3).armour.iter().all(|a| a.0 != ItemKind::Breastplate));
    }
}
