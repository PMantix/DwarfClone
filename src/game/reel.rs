//! The highlight reel (UI-04): the fort's big moments, filmed. When
//! something worth remembering happens (a siege, a beast, a hero falling,
//! an artifact, a death, a wedding) the reel takes a picture of the ground
//! around it and then a few seconds of who moves where, so the Highlights
//! screen (`h`) can play it back with a caption. Sieges get a name, "the
//! Battle of ...", and their clip is retitled with how it ended. At the turn
//! of each year the overseer picks the moment of the year.
//!
//! Clips are small and capped (`MAX_CLIPS`): the ground once as three
//! strings (glyphs, ink, paper), each frame as a string of marks, and every
//! number as one character, and a frame holds at most `MAX_MARKS` marks, so
//! a clip is under 48 KB even in the thick of a siege and a full reel at
//! most about a megabyte of save (usually far less). Filming draws no random
//! numbers, so it never changes a fort's story.

use super::overseer::Overseer;
use super::*;

/// The window filmed around a moment, in tiles.
pub const CLIP_W: i32 = 60;
pub const CLIP_H: i32 = 16;
/// Frames per clip after the first, one every `FRAME_TICKS`.
pub const FRAMES: u8 = 24;
pub const FRAME_TICKS: u64 = 4;
/// The most clips kept; past it the least of them goes.
pub const MAX_CLIPS: usize = 24;
/// The most moments of the year kept starred; past it the least of them
/// loses its star (and may then go like any other clip).
pub const MAX_STARS: usize = 12;
/// The most marks filmed in a frame, however thick the battle.
pub const MAX_MARKS: usize = 120;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub tick: u64,
    pub title: String,
    pub caption: String,
    /// How much it matters (for the cap, and the overseer's pick).
    pub score: u8,
    /// The top-left tile of the window, and its level.
    pub corner: Pos,
    /// Where it happened, inside the window.
    pub at: Pos,
    /// The ground, row by row: one glyph per tile, and its ink and paper
    /// as colour codes (`code`).
    pub ground: String,
    pub ink: String,
    pub paper: String,
    /// Each frame: marks of five characters, x, y, glyph, ink, paper.
    pub frames: Vec<String>,
    /// Frames still to film.
    #[serde(default)]
    pub left: u8,
    /// The overseer's moment of the year.
    #[serde(default)]
    pub star: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reel {
    pub clips: Vec<Clip>,
    /// Deaths and announcements are looked at up to this tick.
    #[serde(default)]
    pub seen: u64,
}

/// A colour as one character: six steps a channel.
pub fn code(c: (u8, u8, u8)) -> char {
    let s = |v: u8| (v as u32 * 5 + 127) / 255;
    char::from_u32(0x100 + s(c.0) * 36 + s(c.1) * 6 + s(c.2)).unwrap_or('Ā')
}

/// And back.
pub fn decode(ch: char) -> (u8, u8, u8) {
    let i = (ch as u32).saturating_sub(0x100).min(215);
    let v = |k: u32| (k * 51) as u8;
    (v(i / 36), v(i / 6 % 6), v(i % 6))
}

/// A small number (a coordinate in the window) as one character.
fn num(n: i32) -> char {
    char::from_u32(0x100 + n.clamp(0, 255) as u32).unwrap_or('Ā')
}

fn unnum(ch: char) -> i32 {
    (ch as u32).saturating_sub(0x100) as i32
}

/// The colour of a ratatui colour, if it's an RGB one.
pub fn rgb_of(c: ratatui::style::Color) -> (u8, u8, u8) {
    match c {
        ratatui::style::Color::Rgb(r, g, b) => (r, g, b),
        _ => (200, 200, 200),
    }
}

/// One thing moving in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub x: i32,
    pub y: i32,
    pub glyph: char,
    pub ink: (u8, u8, u8),
    /// None leaves the ground's paper.
    pub paper: Option<(u8, u8, u8)>,
}

impl Clip {
    /// The ground as rows of (glyph, ink, paper), for drawing.
    pub fn ground_rows(&self) -> Vec<Vec<(char, (u8, u8, u8), (u8, u8, u8))>> {
        let g: Vec<char> = self.ground.chars().collect();
        let ink: Vec<char> = self.ink.chars().collect();
        let paper: Vec<char> = self.paper.chars().collect();
        (0..CLIP_H as usize)
            .map(|y| {
                (0..CLIP_W as usize)
                    .map(|x| {
                        let i = y * CLIP_W as usize + x;
                        (*g.get(i).unwrap_or(&' '), ink.get(i).map_or((0, 0, 0), |c| decode(*c)), paper.get(i).map_or((0, 0, 0), |c| decode(*c)))
                    })
                    .collect()
            })
            .collect()
    }

    /// The marks of frame `i`.
    pub fn marks(&self, i: usize) -> Vec<Mark> {
        let Some(f) = self.frames.get(i) else { return Vec::new() };
        let c: Vec<char> = f.chars().collect();
        c.chunks(5)
            .filter(|m| m.len() == 5)
            .map(|m| Mark { x: unnum(m[0]), y: unnum(m[1]), glyph: m[2], ink: decode(m[3]), paper: (m[4] != ' ').then(|| decode(m[4])) })
            .collect()
    }

    /// Roughly how much save it takes, in bytes.
    pub fn size(&self) -> usize {
        self.ground.len() + self.ink.len() + self.paper.len() + self.frames.iter().map(|f| f.len()).sum::<usize>() + self.title.len() + self.caption.len()
    }
}

/// How much a moment matters, by its card's title.
fn score_of(title: &str) -> u8 {
    match title {
        "A dragon!" | "A beast of legend!" | "The dead walk" => 95,
        t if t.starts_with("The Battle") || t == "Siege!" || t == "The first siege" => 90,
        "A hero falls" | "Their leader falls" => 85,
        "The siege is broken" | "Home in triumph" | "A monument" | "A great work" => 80,
        "An artifact" | "From the deep" | "The hunt for a legend" | "A general caught" => 75,
        "A ghost" | "Cave-in!" | "An execution" | "The magma sea" | "A cavern" => 65,
        "A death" => 55,
        "A wedding" | "Carried from the field" | "A new noble" => 45,
        _ => 35,
    }
}

impl Game {
    /// Start filming a moment (called with every big-moment card, and for
    /// deaths and weddings). The same title again within two days is the
    /// same moment.
    pub(super) fn film(&mut self, title: &str, caption: &str, at: Option<Pos>) {
        if self.world.w <= 1 {
            return;
        }
        let tick = self.tick;
        if let Some(c) = self.reel.clips.iter_mut().rev().find(|c| c.title == title && c.tick + 2 * DAY > tick) {
            // A second death or wedding so soon is a line on the first's
            // clip, not lost; marked "Elsewhere" when it isn't in the shot.
            if matches!(title, "A death" | "A wedding") && !caption.is_empty() && !c.caption.contains(caption) && c.caption.len() + caption.len() < 400 {
                let inside = at.map_or(true, |p| p.z == c.corner.z && p.x >= c.corner.x && p.y >= c.corner.y && p.x < c.corner.x + CLIP_W && p.y < c.corner.y + CLIP_H);
                c.caption.push_str(if inside { " " } else { " Elsewhere: " });
                c.caption.push_str(caption);
            }
            return;
        }
        let first_siege = title == "Siege!" && !self.reel.clips.iter().any(|c| c.title.starts_with("The Battle") || c.title == "Siege!" || c.title == "The first siege");
        let title = if first_siege { "The first siege".to_string() } else { title.to_string() };
        let at = at.filter(|p| self.world.in_bounds(*p)).unwrap_or_else(|| self.reel_focus());
        let corner = Pos::new((at.x - CLIP_W / 2).clamp(0, (self.world.w - CLIP_W).max(0)), (at.y - CLIP_H / 2).clamp(0, (self.world.h - CLIP_H).max(0)), at.z);
        let mut clip = Clip { tick, score: score_of(&title), title, caption: caption.to_string(), corner, at, left: FRAMES, ..Default::default() };
        self.film_ground(&mut clip);
        clip.frames.push(self.film_frame(&clip));
        self.reel.clips.push(clip);
        self.count("highlights", 1);
        // Over the cap: the least (and then oldest) clip that's done filming
        // and isn't a pick goes.
        if self.reel.clips.len() > MAX_CLIPS {
            if let Some(i) = self.reel.clips.iter().enumerate().filter(|(_, c)| c.left == 0 && !c.star).min_by_key(|(_, c)| (c.score, c.tick)).map(|(i, _)| i) {
                self.reel.clips.remove(i);
            }
        }
    }

    #[cfg(test)]
    pub fn film_for_test(&mut self, title: &str, caption: &str, at: Option<Pos>) {
        self.film(title, caption, at);
    }

    /// Where to film when the card says nowhere in particular: the thick of
    /// the hostiles, else the overseer's gate, else the wagon.
    fn reel_focus(&self) -> Pos {
        let foes: Vec<Pos> = self.creatures.values().filter(|c| c.alive && c.hostile()).map(|c| c.pos).collect();
        if let Some(&p) = foes.first() {
            let n = foes.len() as i32;
            let (sx, sy) = foes.iter().fold((0, 0), |a, q| (a.0 + q.x, a.1 + q.y));
            return Pos::new(sx / n, sy / n, p.z);
        }
        self.overseer.as_ref().and_then(|o| o.base).unwrap_or(self.embark)
    }

    /// The ground in the window, as the map draws it bare (with buildings
    /// and things lying about).
    fn film_ground(&self, clip: &mut Clip) {
        let mut items: HashMap<(i32, i32), (char, (u8, u8, u8))> = HashMap::new();
        let c0 = clip.corner;
        for it in self.items.values().filter(|it| it.pos.z == c0.z && it.carried_by.is_none() && it.container.is_none()) {
            let (x, y) = (it.pos.x - c0.x, it.pos.y - c0.y);
            if x >= 0 && y >= 0 && x < CLIP_W && y < CLIP_H {
                let (g, col) = it.glyph();
                items.entry((x, y)).or_insert((g, rgb_of(col)));
            }
        }
        for y in 0..CLIP_H {
            for x in 0..CLIP_W {
                let p = Pos::new(c0.x + x, c0.y + y, c0.z);
                let (mut g, mut ink, mut paper) = if self.world.in_bounds(p) {
                    let (g, i, pa) = crate::ui::tile_look(&self.world, p);
                    (g, rgb_of(i), rgb_of(pa))
                } else {
                    (' ', (0, 0, 0), (0, 0, 0))
                };
                if self.world.in_bounds(p) && self.world.t(p).revealed {
                    let t = self.world.t(p);
                    if t.building != 0 {
                        if let Some(b) = self.buildings.get(&t.building) {
                            g = b.glyph_at(p.x - b.pos.x, p.y - b.pos.y);
                            ink = if !b.built { (90, 90, 90) } else if b.kind.is_workshop() { rgb_of(b.kind.color()) } else { rgb_of(b.mat.color()) };
                        }
                    } else if let Some(&(ig, ic)) = items.get(&(x, y)) {
                        g = ig;
                        ink = ic;
                    }
                    if !t.kind.solid() {
                        if let Some(s) = self.gore.stains.get(&p) {
                            paper = s.rgb();
                        }
                    }
                }
                clip.ground.push(g);
                clip.ink.push(code(ink));
                clip.paper.push(code(paper));
            }
        }
    }

    /// Who and what is moving in the window now.
    fn film_frame(&self, clip: &Clip) -> String {
        let c0 = clip.corner;
        let mut s = String::new();
        let mut n = 0;
        let mut put = |p: Pos, g: char, ink: (u8, u8, u8), paper: Option<(u8, u8, u8)>| {
            let (x, y) = (p.x - c0.x, p.y - c0.y);
            if n < MAX_MARKS && p.z == c0.z && x >= 0 && y >= 0 && x < CLIP_W && y < CLIP_H {
                n += 1;
                s.push(num(x));
                s.push(num(y));
                s.push(g);
                s.push(code(ink));
                s.push(paper.map_or(' ', code));
            }
        };
        for (p, at) in &self.flames {
            if at + 8 > self.tick {
                put(*p, '☼', (255, 230, 120), Some((200, 60, 0)));
            }
        }
        for it in self.items.values().filter(|it| it.kind == ItemKind::Corpse && it.pos.z == c0.z && it.carried_by.is_none()) {
            let (g, col) = it.glyph();
            put(it.pos, g, rgb_of(col), None);
        }
        for c in self.creatures.values().filter(|c| c.alive && !c.sneaking) {
            let info = c.species.info();
            put(c.pos, info.glyph, info.color, c.hostile().then_some((90, 0, 0)));
        }
        for d in self.dwarves.iter().filter(|d| d.alive) {
            let glyph = if d.soldier { '☻' } else { '☺' };
            put(d.pos, glyph, d.color, (d.body.bleeding > 0).then_some((70, 0, 0)));
        }
        s
    }

    /// Every few ticks: film the next frame of each clip still rolling,
    /// and look for deaths and weddings since last time.
    pub(super) fn reel_tick(&mut self) {
        if self.reel.clips.iter().any(|c| c.left > 0) {
            let frames: Vec<(usize, String)> = self.reel.clips.iter().enumerate().filter(|(_, c)| c.left > 0).map(|(i, c)| (i, self.film_frame(c))).collect();
            for (i, f) in frames {
                let c = &mut self.reel.clips[i];
                c.frames.push(f);
                c.left -= 1;
            }
        }
        if self.tick % (FRAME_TICKS * 5) != 1 {
            return;
        }
        let since = self.reel.seen;
        self.reel.seen = self.tick;
        // A save from before the reel: start watching from now, not from
        // the founding.
        if since == 0 && self.tick > DAY {
            return;
        }
        // Deaths: a clip each, or a line on a clip still rolling there.
        let died: Vec<(String, Pos)> = self.dwarves.iter().filter(|d| !d.alive && d.died.map_or(false, |t| t > since)).map(|d| (d.name.clone(), d.pos)).collect();
        for (name, p) in died {
            let cause = self.annals.deaths.iter().rev().find(|n| n.name == name).map_or("died".to_string(), |n| match &n.killer {
                Some(k) if n.cause != *k => format!("{} ({})", n.cause, k),
                _ => n.cause.clone(),
            });
            let line = format!("{} {}.", name, cause);
            let rolling = self.reel.clips.iter_mut().rev().find(|c| c.left > 0 && c.corner.z == p.z && p.x >= c.corner.x && p.y >= c.corner.y && p.x < c.corner.x + CLIP_W && p.y < c.corner.y + CLIP_H);
            match rolling {
                Some(c) => {
                    if !c.caption.contains(&name) && c.caption.len() < 400 {
                        c.caption.push(' ');
                        c.caption.push_str(&line);
                    }
                }
                None => self.film("A death", &line, Some(p)),
            }
        }
        // Weddings, from the announcements.
        let weds: Vec<String> = self.log.iter().rev().take_while(|l| l.tick > since).filter(|l| l.text.ends_with("have married.")).map(|l| l.text.clone()).collect();
        for text in weds {
            let first = text.split(" and ").next().unwrap_or("").to_string();
            let at = self.dwarves.iter().find(|d| d.name == first).map(|d| d.pos);
            self.film("A wedding", &text, at);
        }
    }

    /// A siege has ended: its clip gets the battle's name and how it went.
    pub(super) fn reel_siege_end(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i).cloned() else { return };
        let name = self.battle_name(s.tick);
        let Some(c) = self.reel.clips.iter_mut().find(|c| (c.title == "Siege!" || c.title == "The first siege") && c.tick + DAY >= s.tick && c.tick <= s.tick + 2 * DAY) else { return };
        let first = c.title == "The first siege";
        c.title = name.clone();
        c.score = 90;
        let lost = match s.dwarves_lost {
            0 => "no dwarf lost".to_string(),
            1 => "one dwarf lost".to_string(),
            n => format!("{} dwarves lost", n),
        };
        c.caption = format!("{}{}: the {} {}, {} slain, {}. {}", name, if first { ", the first siege" } else { "" }, s.foe, s.outcome, s.foes_slain, lost, c.caption);
        c.caption.truncate(c.caption.char_indices().nth(400).map_or(c.caption.len(), |(i, _)| i));
    }

    /// A name for the battle of a siege that came on `tick`: after the
    /// ground it was fought on, without drawing on the fort's dice.
    pub fn battle_name(&self, tick: u64) -> String {
        const PLACES: [&str; 8] = ["Gate", "Stair", "Fields", "Wagon", "Long Wall", "Red Mud", "Broken Shields", "Cold Hearth"];
        let h = (tick / DAY).wrapping_mul(2654435761) ^ (self.reel.clips.len() as u64);
        format!("The Battle of the {}", PLACES[(h % PLACES.len() as u64) as usize])
    }

    /// Daily, from the overseer's round: on the first day of the year it
    /// picks the moment of the year gone by and says so.
    pub(super) fn ov_reel(&mut self, o: &mut Overseer) {
        if self.tick < YEAR || (self.tick % YEAR) / DAY != 0 {
            return;
        }
        let from = self.tick - YEAR;
        let Some(c) = self.reel.clips.iter_mut().filter(|c| c.tick >= from && c.left == 0).max_by_key(|c| (c.score, c.tick)) else { return };
        if c.star {
            return;
        }
        c.star = true;
        let text = format!("The moment of the year: {} ({}).", c.title.trim_end_matches('!'), first_words(&c.caption, 8));
        Self::note_ov(o, self.tick, text);
        self.count("highlights_starred", 1);
        // Too many stars: the least (and then oldest) loses its star, so
        // the cap on clips still holds in a long fort.
        if self.reel.clips.iter().filter(|c| c.star).count() > MAX_STARS {
            if let Some(c) = self.reel.clips.iter_mut().filter(|c| c.star && c.tick < from).min_by_key(|c| (c.score, c.tick)) {
                c.star = false;
            }
        }
    }
}

/// The first `n` words, with an ellipsis if there were more.
fn first_words(s: &str, n: usize) -> String {
    let w: Vec<&str> = s.split_whitespace().collect();
    if w.len() <= n {
        s.trim_end_matches('.').to_string()
    } else {
        format!("{}...", w[..n].join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_numbers_fit_in_a_character() {
        for c in [(0, 0, 0), (255, 255, 255), (255, 60, 60), (90, 160, 255)] {
            let back = decode(code(c));
            assert!((back.0 as i32 - c.0 as i32).abs() <= 26 && (back.1 as i32 - c.1 as i32).abs() <= 26 && (back.2 as i32 - c.2 as i32).abs() <= 26, "{:?} -> {:?}", c, back);
        }
        assert_eq!(unnum(num(59)), 59);
    }

    #[test]
    fn a_big_moment_is_filmed_and_played_back() {
        let mut g = Game::new(5);
        let at = g.dwarves[0].pos;
        g.spotlight("An artifact", "Urist has created a thing of beauty!", Some(at), (255, 215, 60));
        assert_eq!(g.reel.clips.len(), 1);
        let c = &g.reel.clips[0];
        assert_eq!(c.ground.chars().count(), (CLIP_W * CLIP_H) as usize);
        assert_eq!(c.frames.len(), 1);
        // The dwarf is in the first frame, where they stand.
        let m = c.marks(0);
        assert!(m.iter().any(|m| (m.x, m.y) == (at.x - c.corner.x, at.y - c.corner.y) && (m.glyph == '☺' || m.glyph == '☻')), "{:?}", m);
        // It rolls on for the frames to come, and then stops.
        for _ in 0..(FRAMES as u64 + 4) * FRAME_TICKS {
            g.step();
        }
        let c = &g.reel.clips[0];
        assert_eq!((c.frames.len(), c.left), (FRAMES as usize + 1, 0));
        // Small enough to keep a reel of them.
        assert!(c.size() < 16_000, "{} bytes", c.size());
        // And it saves.
        let back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back.reel.clips[0].frames, g.reel.clips[0].frames);
    }

    #[test]
    fn deaths_and_weddings_are_filmed_and_the_reel_is_capped() {
        let mut g = Game::new(5);
        g.tick += 10;
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.kill_for_test(&mut d, "was struck down");
        let name = d.name.clone();
        g.dwarves[1] = d;
        g.msg("Urist and Kadol have married.", (255, 200, 220));
        while g.tick % (FRAME_TICKS * 5) != 1 {
            g.tick += 1;
        }
        g.reel_tick();
        assert!(g.reel.clips.iter().any(|c| c.title == "A death" && c.caption.contains(&name)), "{:?}", g.reel.clips.iter().map(|c| &c.title).collect::<Vec<_>>());
        assert!(g.reel.clips.iter().any(|c| c.title == "A wedding"));
        // Another wedding the next day is a line on the same clip.
        g.tick += DAY;
        g.film("A wedding", "Bomrek and Ingiz have married.", None);
        let weds: Vec<&Clip> = g.reel.clips.iter().filter(|c| c.title == "A wedding").collect();
        assert_eq!(weds.len(), 1);
        assert!(weds[0].caption.contains("Urist and Kadol") && weds[0].caption.contains("Bomrek and Ingiz"), "{}", weds[0].caption);
        for i in 0..40 {
            g.tick += 3 * DAY;
            g.reel.clips.iter_mut().for_each(|c| c.left = 0);
            g.film(&format!("Moment {}", i), "", None);
        }
        assert_eq!(g.reel.clips.len(), MAX_CLIPS);
        assert!(g.reel.clips.iter().any(|c| c.title == "A death"), "the weightier moments stay");
    }

    /// An autoplay fort's first siege (seed 5003's, on day 253) is on its
    /// reel, filmed frame by frame.
    #[test]
    fn an_autoplay_fort_films_its_first_siege() {
        let g = trial_game(5003, Style::Steady, 262);
        let c = g.reel.clips.iter().find(|c| c.title == "The first siege" || c.title.starts_with("The Battle")).expect("the siege is on the reel");
        assert_eq!(c.frames.len(), FRAMES as usize + 1);
        assert!(c.frames.iter().any(|f| !f.is_empty()), "someone moves in it");
        assert!(g.annals.counters.get("highlights").copied().unwrap_or(0) >= 1);
    }

    #[test]
    fn a_siege_is_named_and_the_overseer_picks_the_moment_of_the_year() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        g.tick = 100;
        g.spotlight("Siege!", "Goblins!", None, (255, 70, 70));
        assert_eq!(g.reel.clips[0].title, "The first siege");
        g.annals.sieges.push(annals::SiegeNote { tick: 100, foe: "goblins".into(), size: 5, outcome: "routed".into(), foes_slain: 4, ..Default::default() });
        g.reel_siege_end(0);
        let c = &g.reel.clips[0];
        assert!(c.title.starts_with("The Battle of the "), "{}", c.title);
        assert!(c.caption.contains("the first siege") && c.caption.contains("routed"), "{}", c.caption);
        g.spotlight("Traders", "Merchants.", None, (240, 240, 160));
        g.reel.clips.iter_mut().for_each(|c| c.left = 0);
        g.tick = YEAR;
        let mut o = g.overseer.take().unwrap();
        g.ov_reel(&mut o);
        assert!(g.reel.clips[0].star && !g.reel.clips[1].star);
        assert!(o.notes.iter().any(|n| n.1.starts_with("The moment of the year: The Battle of")), "{:?}", o.notes);
        g.overseer = Some(o);
    }

    /// A long fort stars a moment a year, but keeps at most `MAX_STARS`
    /// stars, so the reel stays within its cap.
    #[test]
    fn stars_are_capped_and_so_is_the_reel() {
        let mut g = Game::new(5);
        let mut o = Overseer::new(Style::Steady);
        for y in 1..=40u64 {
            g.tick = y * YEAR - 30 * DAY;
            g.film(&format!("A great work {}", y), "", None);
            g.reel.clips.iter_mut().for_each(|c| c.left = 0);
            g.tick = y * YEAR;
            g.ov_reel(&mut o);
            assert!(g.reel.clips.iter().filter(|c| c.star).count() <= MAX_STARS);
            assert!(g.reel.clips.len() <= MAX_CLIPS, "{} clips in year {}", g.reel.clips.len(), y);
        }
        // This year's pick keeps its star.
        assert!(g.reel.clips.iter().any(|c| c.star && c.title == "A great work 40"));
    }

    /// A second death far from the first clip is noted as elsewhere, not
    /// as if the clip showed it.
    #[test]
    fn a_death_out_of_shot_is_elsewhere() {
        let mut g = Game::new(5);
        let at = g.dwarves[0].pos;
        g.film("A death", "Urist was struck down.", Some(at));
        let far = Pos::new((at.x + CLIP_W * 2).min(g.world.w - 1), at.y, at.z);
        let far = if (far.x - g.reel.clips[0].corner.x) < CLIP_W { Pos::new(at.x, at.y, at.z - 1) } else { far };
        g.film("A death", "Kadol was struck down.", Some(far));
        g.film("A death", "Bomrek was struck down.", Some(at));
        let c = &g.reel.clips[0];
        assert!(c.caption.contains("Elsewhere: Kadol") && c.caption.contains(" Bomrek") && !c.caption.contains("Elsewhere: Bomrek"), "{}", c.caption);
    }

    /// A siege packed into the window stays small: frames hold at most
    /// `MAX_MARKS` marks, and a full reel of such clips is about a
    /// megabyte at the very most.
    #[test]
    fn a_crowded_siege_clip_stays_small() {
        let mut g = Game::new(5);
        let at = g.dwarves[0].pos;
        let mut n = 0;
        for dy in -7..7 {
            for dx in -28..28 {
                let p = Pos::new(at.x + dx, at.y + dy, at.z);
                if (dx + dy) % 3 == 0 && g.world.in_bounds(p) && !g.world.t(p).kind.solid() {
                    g.spawn_creature(crate::creature::Species::Goblin, p, u64::MAX);
                    n += 1;
                }
            }
        }
        assert!(n > MAX_MARKS, "only {} goblins", n);
        g.film("Siege!", "Goblins!", Some(at));
        for _ in 0..FRAMES {
            let f = g.film_frame(&g.reel.clips[0]);
            let c = &mut g.reel.clips[0];
            c.frames.push(f);
            c.left -= 1;
        }
        let c = &g.reel.clips[0];
        assert!(c.marks(0).len() == MAX_MARKS);
        let json = serde_json::to_string(c).unwrap().len();
        println!("a crowded siege clip: {} bytes of save", json);
        assert!(json < 48_000, "{} bytes", json);
        assert!(json * MAX_CLIPS < 1_200_000);
    }
}
