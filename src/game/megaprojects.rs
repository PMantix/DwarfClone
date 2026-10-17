//! Megaprojects: a great monument raised block by block over a season or
//! more, dedicated to the fortress's greatest hero with a feast.

use super::*;
use super::festivals::FestivalKind;

impl Game {
    /// The monument is finished: name it for the greatest of the fortress,
    /// and hold a feast.
    pub(super) fn dedicate_monument(&mut self, bld: u32, builder: &Dwarf) {
        let fort = self.fortress_name.clone();
        // Everyone, including whoever set the last block (out of the list
        // for their own turn).
        let all: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.id != builder.id).chain(std::iter::once(builder)).collect();
        let hero = all.iter().filter(|d| d.epithet.is_some()).max_by_key(|d| (d.epithet_rank, d.kills, d.id)).map(|d| d.called());
        let leader = all.iter().filter(|d| d.alive && d.noble.is_some()).map(|d| d.called()).next();
        let name = match (hero, leader) {
            (Some(h), _) => format!("the Statue of {}", h),
            (None, Some(l)) => format!("the Statue of {}", l),
            (None, None) => format!("the Great Obelisk of {}", fort),
        };
        let mat = self.buildings.get(&bld).map_or(Mat::Granite, |b| b.mat);
        self.monument = Some(name.clone());
        self.year_log.monument = Some(name.clone());
        self.msg(&format!("{} stands complete, carved in {}! It will be remembered.", cap_first(&name), mat.name()), (255, 215, 60));
        self.legend(format!("The dwarves of {} raised {}, of {}.", fort, name, mat.name()));
        self.milestone("monument", &format!("Raised {}", name));
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive) {
            d.think(&format!("was proud to see {} raised", name), -30, tick);
        }
        self.festival = None;
        self.start_festival(FestivalKind::Dedication);
        let at = self.buildings.get(&bld).map(|b| b.pos);
        self.spotlight("A monument", &format!("{} stands complete! It will be remembered.", cap_first(&name)), at, (255, 215, 60));
    }

    /// Daily: dwarves who pass the monument stop to admire it.
    pub(super) fn monument_daily(&mut self) {
        let Some(name) = self.monument.clone() else { return };
        let Some(at) = self.buildings.values().find(|b| b.kind == BuildingKind::Monument && b.built).map(|b| b.center()) else { return };
        let tick = self.tick;
        let text = format!("admired {}", name);
        for d in self.dwarves.iter_mut().filter(|d| d.alive && (d.pos.z - at.z).abs() <= 1 && (d.pos.x - at.x).abs() <= 8 && (d.pos.y - at.y).abs() <= 8) {
            if !d.thoughts.iter().any(|t| t.text == text && tick < t.tick + DAY * 3) {
                d.think(&text, -8, tick);
            }
        }
    }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::MONUMENT_BLOCKS;

    #[test]
    fn a_monument_rises_block_by_block_and_is_dedicated() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let site = (3..30)
            .flat_map(|r| (-r..=r).flat_map(move |k| [e.offset(k, -r, 0), e.offset(k, r, 0), e.offset(-r, k, 0), e.offset(r, k, 0)]))
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| g.can_place(BuildingKind::Monument, p) && crate::path::astar(&g.world, e, p, 20_000, crate::world::Mover::Dwarf).is_some())
            .expect("nowhere to build");
        assert!(g.place_building(BuildingKind::Monument, site));
        let id = g.world.t(site).building;
        for _ in 0..MONUMENT_BLOCKS + 5 {
            g.spawn_item(ItemKind::Block, Mat::Granite, e);
        }
        let hero = g.dwarves[2].id;
        g.dwarves[2].epithet = Some("Dragonslayer".into());
        g.dwarves[2].epithet_rank = 5;
        for _ in 0..DAY * 12 {
            g.step();
            for d in g.dwarves.iter_mut() {
                d.hunger = 0;
                d.thirst = 0;
            }
            if g.buildings.get(&id).map_or(false, |b| b.built) {
                break;
            }
        }
        let b = g.buildings.get(&id).expect("the monument vanished");
        assert!(b.built, "the monument was never finished ({} of {} blocks)", b.parts, MONUMENT_BLOCKS);
        let name = g.monument.clone().unwrap();
        let first = g.dwarves.iter().find(|d| d.id == hero).unwrap().called();
        assert_eq!(name, format!("the Statue of {}", first));
        assert!(g.milestones.iter().any(|m| m.0 == "monument"));
        assert!(g.festival.as_ref().map_or(false, |f| f.kind == FestivalKind::Dedication));
        assert!(g.building_value(b) >= 400, "a monument is worth a great deal");
    }
}
