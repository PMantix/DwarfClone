//! The job finders: what an idle dwarf looks for once their needs are met,
//! in order of priority. `choose_task` walks the table and takes the first
//! finder that gives them a task. A new kind of work adds a row here.

use super::*;

/// Looks for work for a dwarf; true if it gave them a task.
pub(super) type Finder = fn(&mut Game, &mut Dwarf) -> bool;

/// Every finder, most urgent first.
pub(super) const FINDERS: &[(&str, Finder)] = &[
    ("punishment", |g, d| g.find_punishment(d)),
    ("rescue", |g, d| g.find_rescue(d)),
    ("lever", |g, d| g.find_lever_pull(d)),
    ("first aid", |g, d| g.find_first_aid(d)),
    ("patient", |g, d| d.has(Labor::Medicine) && g.find_patient(d)),
    ("coax", |g, d| g.find_coax(d)),
    ("slaughter", |g, d| d.has(Labor::Butchery) && g.find_slaughter(d)),
    ("burial", |g, d| d.has(Labor::Hauling) && g.find_burial(d)),
    ("construct", |g, d| d.has(Labor::Building) && (g.find_construct(d) || g.find_build_tile(d))),
    ("craft", |g, d| g.find_craft(d)),
    ("detail", |g, d| d.has(Labor::Engraving) && g.find_detail(d)),
    ("mine", |g, d| d.has(Labor::Mining) && g.find_dig(d, false)),
    // Outdoor work waits while beasts prowl near (hunts.rs).
    ("chop", |g, d| d.has(Labor::Woodcutting) && !g.outdoors_banned() && g.find_dig(d, true)),
    ("farm", |g, d| d.has(Labor::Farming) && (g.find_farm(d) || !g.outdoors_banned() && g.find_gather(d))),
    ("fish", |g, d| d.has(Labor::Fishing) && !g.outdoors_banned() && g.find_fishing(d)),
    // Drink and meals into the shelter's larder (shelter.rs).
    ("larder", |g, d| d.has(Labor::Hauling) && !d.specialist && g.find_larder_haul(d)),
    // Specialists don't haul; the depot and the dump come before the stockpiles.
    ("haul", |g, d| d.has(Labor::Hauling) && !d.specialist && (g.find_depot_haul(d) || g.find_dump_haul(d) || g.find_haul(d) || g.find_clean(d))),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finders_are_named_once() {
        let mut names: Vec<&str> = FINDERS.iter().map(|f| f.0).collect();
        assert_eq!(names.first(), Some(&"punishment"));
        assert_eq!(names.last(), Some(&"haul"));
        names.sort();
        names.dedup();
        assert_eq!(names.len(), FINDERS.len());
    }
}
