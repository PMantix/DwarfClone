//! Fitting out a freshly dug room, one function per kind of room (M2
//! W0-B: the arms of `overseer.rs: ov_fit`, moved verbatim, so O03 and O11
//! edit different functions). Each places what the room holds and returns
//! the zone to lay over it, if any.

use super::layout::Shape;
use super::overseer::{Overseer, Room, WORKSHOPS};
use super::*;

impl Game {
    pub(super) fn fit_bedrooms(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        for &p in &r.beds {
            self.place_building(BuildingKind::Bed, p);
        }
        None
    }

    pub(super) fn fit_workshops(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        let have: Vec<BuildingKind> = self.buildings.values().map(|b| b.kind).collect();
        let todo: Vec<BuildingKind> = WORKSHOPS.iter().copied().filter(|k| !have.contains(k)).collect();
        let mut placed = 0;
        for (dx, dy) in [(0, 0), (4, 0), (0, 3), (4, 3), (8, 0), (8, 3)] {
            let Some(&kind) = todo.get(placed) else { break };
            let p = r.a.offset(dx, dy, 0);
            if p.x + 2 <= r.b.x && p.y + 2 <= r.b.y && self.place_building(kind, p) {
                placed += 1;
            }
        }
        None
    }

    pub(super) fn fit_dining(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.ov_grid(r, BuildingKind::Table, 3, 2, 12);
        let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && box_iter(r.a, r.b).any(|p| p == b.pos)).map(|b| b.pos).collect();
        for t in tables {
            self.place_building(BuildingKind::Chair, t.offset(1, 0, 0));
        }
        Some(ZoneKind::Dining)
    }

    pub(super) fn fit_dormitory(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.ov_grid(r, BuildingKind::Bed, 2, 2, 12);
        Some(ZoneKind::Dormitory)
    }

    pub(super) fn fit_tavern(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.ov_grid(r, BuildingKind::Table, 3, 3, 4);
        Some(ZoneKind::Tavern)
    }

    pub(super) fn fit_hospital(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.ov_grid(r, BuildingKind::Bed, 3, 3, 3);
        // Its last row keeps the thread, cloth and splints to hand.
        self.add_zone(Pos::new(r.a.x, r.b.y, r.a.z), r.b, ZoneKind::Stockpile(StockFilter::Medical));
        Some(ZoneKind::Hospital)
    }

    pub(super) fn fit_tomb(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.ov_grid(r, BuildingKind::Coffin, 2, 2, 8);
        Some(ZoneKind::Tomb)
    }

    pub(super) fn fit_barracks(&mut self, _o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        self.place_building(BuildingKind::ArcheryTarget, r.a);
        Some(ZoneKind::Barracks)
    }

    pub(super) fn fit_grand(&mut self, o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        match r.shape {
            // A great hall to feast in, among the pillars.
            Some(Shape::Pillared { .. }) => {
                self.ov_grid(r, BuildingKind::Table, 3, 2, 20);
                let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && r.tiles.contains(&b.pos)).map(|b| b.pos).collect();
                for t in tables {
                    self.place_building(BuildingKind::Chair, t.offset(1, 0, 0));
                }
                Some(ZoneKind::Dining)
            }
            // A rotunda with the monument at its heart.
            Some(Shape::Round { r: rad }) => {
                if !o.did("monument") && self.place_building(BuildingKind::Monument, r.a.offset(rad - 1, rad - 1, 0)) {
                    o.done.push("monument".into());
                }
                Some(ZoneKind::Meeting)
            }
            // A cross: a temple.
            Some(Shape::Cross { .. }) => Some(ZoneKind::Temple),
            // The long gallery: every wall carved.
            _ => {
                self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Smooth);
                Some(ZoneKind::Meeting)
            }
        }
    }
}
