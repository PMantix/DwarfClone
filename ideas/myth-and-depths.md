# Myth and the depths: religion, creatures, caverns

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## Religion, myth & magic (MYT)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Gods | Generated deities with spheres; each dwarf worships particular gods with a strength of devotion | None; a single generic Pray need (`src/game/needs.rs`) | No gods at all; prayer has no object |
| Temples | Temples dedicated to one god or to all; value tiers unlock a priest, then a temple complex and high priest | Temple zone meets the Pray need; pious overseers dig temples; Architect's cross-shaped temple (`needs.rs`, `layout.rs`) | No dedication, tiers, priests or sermons |
| Petitions | Enough believers ask for a temple; refusing an accepted petition angers them | None | Missing; perfect overseer decision point (Pious trait) |
| Ghosts | Unburied or unremembered dead return; varied behaviour (violent, poltergeist, haunting), tied to how they died | Ghosts of the unburied drift and frighten until buried or memorialised (`src/game/family.rs` `ghosts_tick`) | One behaviour only |
| Necromancy | Necromancers learn from slabs or gods, build towers, raise local dead, run experiments, field undead lieutenants | One necromancer per site raises a horde and the fort's unburied dead; slaying it stills them (`src/game/necromancy.rs`) | No slabs, experiments, lieutenants or spreading evil |
| Undead | Severed parts rise separately; pulping a key part ends them; risen kin are recognised | Risen dwarves fall back to corpses (`necromancy.rs`); husks on evil land (`creature.rs` Husk) | No parts, no recognition, no dormant dead |
| Evil regions | Evil weather and mists husk victims, raise the dead, spread from towers and fade | Blood rain upsets, husks spawn on evil land (`src/game/weather.rs`) | No cause, no spread, no cleansing |
| Vampires | Cursed citizens arrive with false identities, never age, drain sleepers | None (Inbox) | Missing |
| Werebeasts | Raid under the full moon; the bitten transform at the next one | None (Inbox) | Missing; no moon phase either |
| Curses | Desecration or a god's anger curses a dwarf; framework of syndromes tied to moon and sun | None | Missing |
| Tombs & mummies | Robbed tombs wake a mummy that raises the buried and lays curses | Sealed ancient vault wakes a unique warden when opened (`src/game/ruins.rs`) | Warden only; no tomb curse or raised dead |
| Divination & omens | Dice shrines give omens; beast cults grow around repeated attacks | Mysteries: Whispering Stone and hooded stranger (`src/game/mysteries.rs`) | No omens, dice, cults |
| Artifacts | Mostly valuable; planned rare powers, intelligence and factions chasing them | Named quality-6 artifacts from moods, value only (`items.rs`, `psyche.rs`) | No powers or curses |
| Magic & wizards | Upcoming: generated magic, wizard residents with labs, magical sieges, divine powers | None (WISHLIST B14) | Whole arc missing |
| Myths | Generated creation myths and cosmologies | Legends and world history only (`overworld.rs`) | No myth layer |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| MYT-01 | Richer ghosts | Ghosts behave by how they lived and died: act out their old job, keep their grudges, guide the living to their body or lie to them, and haunt whoever they blame (the lever-puller who flooded them). Violent ghosts strike, poltergeists move objects. | Partial: ghosts of the unburied frighten until buried (family.rs) | S | ★★★ | B23 | Bloat#21, Arc Ghosts, Inbox (violent ghosts), Story:A Terror to Behold |
| MYT-02 | Horror at undead kin | A risen corpse keeps its name; spouses and children who see it take a heavy shock, and its ghost may walk at the same time. | Partial: own dead rise in necromancer raids | S | ★★★ | B23 | Log2006-19 |
| MYT-03 | Werebeast raid and moon curse | A moon phase drives a werebeast to rush the fort, maul a few dwarves and flee; the bitten change at the next full moon and attack their friends, then wake ashamed. | Planned: Inbox (werebeasts) | M | ★★★ | N01 | Log2006-19 (Rel0.34), Core96, Night-creature role 2012-2020, PG60, PG97, Story:Root, Story:catalogue (werebeasts) |
| MYT-04 | Secret vampire citizen | A migrant with a false history never ages, drinks from sleepers (a timed latched bite), grows sloppy with thirst until someone dies, hides the body, and gives itself away by fleeing in a fight. | Planned: Inbox (vampires) | M | ★★★ | N01 | Log2006-19 (Rel0.34), Log2020-26 (divine curses), Bloat#249, Core96, Story:Cado's Magical Journey |
| MYT-05 | Omens | A black stag, a blood moon, birds falling silent or a storm from nowhere precede a beast or siege by days; superstitious dwarves fret, and the overseer can read them as a sense. | Missing | S | ★★★ | N01 | Story:Orther Pushes His Luck, Story:Moclem, Story:catalogue |
| MYT-06 | Banes and weaknesses | Supernatural foes have a known answer (silver, steel, fire, dawn, running water, a song); dwarves learn it the hard way and the fort remembers it. | Missing | S | ★★★ | N01 | Log2006-19 (silver), Story:A Terror to Behold, Story:Heroes of the Coast, Story:Cold Fire, Story:catalogue |
| MYT-07 | Bogeymen | Shapes that stalk a dwarf alone outside at night, in a few cursed regions; a reason to come home before dark. | Missing | S | ★★★ | N01 | 2020, Log2020-26 |
| MYT-08 | Mummies and tomb curses | Breaking into a tomb wakes a mummy that raises the buried and lays a curse (bad luck, sickness) on the looters. | Partial: sealed vault wakes a warden (ruins.rs) | S | ★★★ | B23 | Log2006-19 (Rel0.40), Log2020-26 |
| MYT-09 | Evil with a cause, evil purged | An evil region has a figure behind it; killing it (or another deed) lifts blood rain and husking, and the region's fall into evil and old name enter history. | Partial: slaying the necromancer stills the dead (necromancy.rs) | M | ★★★ | B23 | Bloat#238, Bloat#330, Bloat#239 |
| MYT-10 | Gods with spheres | Each civilization has gods with spheres; dwarves worship particular ones with a strength of devotion and may pray to any god they know of. | Planned: Inbox (deities), B14 | M | ★★ | C10 | Bloat#376, Bloat#378, Bloat#381, roadmap Gods with spheres, Log2006-19, Story:catalogue |
| MYT-11 | Temples, priests and sermons | A temple dedicated to a god; once grand enough, a priest is appointed who preaches, cheering sympathisers and easing the stressed. | Partial: temple zone and Pray need; Planned: B08 | M | ★★★ | C10 | Log2020-26, Log2006-19 (Rel0.47) |
| MYT-12 | Religious petitions | When enough dwarves share a faith they petition for a temple, later a complex and high priest; accepting and then not building angers them. The overseer answers by its traits. | Planned: B09 | M | ★★★ | C10 | Log2006-19 (Rel0.47), Log2020-26 (petitions) |
| MYT-13 | Desecration and divine curses | A dwarf who smashes a temple's statue in a tantrum, or breaks a vow, may be cursed by that god into a vampire or werebeast. | Missing | S | ★★★ | C10 | Log2006-19 (Rel0.34), Log2020-26 (divine curses), PG113 |
| MYT-14 | Beast cults | A people (or a single dwarf) beaten by the same beast again and again comes to fear and worship it, with drifting superstitions and, rarely, a shrine. | Missing | M | ★★★ | C10 | Log2020-26, Log2006-19 |
| MYT-15 | Answered prayers | Rarely, a desperate prayer that fits a god's sphere is answered: a sign, a healing, a storm on the besiegers, a relic handed to a prophet. | Planned: B14 | M | ★★★ | C10 | PG109, PG113, PG139, Story:Gorge of the Righteous, Story:catalogue |
| MYT-16 | Undead from parts | Severed limbs, heads and hides rise on their own on evil land; pulping the key part puts a corpse down for good. | Missing | M | ★★★ | B23 | Log2006-19 (Rel0.31) |
| MYT-17 | Dormant dead | Undead stand idle until disturbed; a quiet horde in a ruin or cavern waits for the miner who breaks through. | Partial: undead raids only | S | ★★ | B23 | Log2020-26 |
| MYT-18 | Necromancer escalation | Necromancers experiment on captives (warped monsters, fused giants that later wander in), raise intelligent lieutenants, summon nightmares, spread evil zones and promise immortality. | Partial: F14 necromancer | L | ★★★ | B23 | Log2020-26, Log2006-19 (experiments), Log2020-26 (undead and demon sieges) |
| MYT-19 | Secrets of death in the fort | A dwarf who fears death finds a slab of dark lore (in a ruin, by trade) and turns necromancer inside the fort; a looted slab in a library corrupts its readers. | Missing | M | ★★★ | B23 | Log2006-19 (Rel0.40) |
| MYT-20 | Spreading evil land | Evil fogs and rains turn the caught into husks, spread from towers and fade with time. | Partial: blood rain, husks spawn (weather.rs) | M | ★★★ | B23 | Log2006-19, Log2020-26 (evil weather), PG116, PG167, PG183 |
| MYT-21 | Cursed artifacts | A relic spreads sickness or bad luck until destroyed or carried to a set place. | Missing | M | ★★★ | N03 | Story:Curse of the Cyclops, Story:catalogue |
| MYT-22 | Artifact powers | Rare artifacts with mysterious powers (ward against blades, ever-full healing cup, thought-reading helm); some are intelligent; all draw factions. | Missing; Planned: B05 (politics) | M | ★★★ | N03 | Core80, Arc Artifact, 2020, Story:Heroes of the Coast, Story:catalogue |
| MYT-23 | Cursed sites | The site itself is cursed; lifting it takes investigation, a hard choice or moving an artifact. | Missing | M | ★★★ | N03 | 2020 |
| MYT-24 | Divination shrine | Dice on a pedestal; a roll gives an omen with an effect, sometimes a curse; groundwork for gambling. | Missing | S | ★★ | C10 | Log2020-26, Log2006-19 (Rel0.47) |
| MYT-25 | Charms and wards | Dwarves paint wards on doors and carry charms; whether they work depends on the world's myth. | Missing | S | ★★ | N01 | Story:A Terror to Behold, Story:Orther Pushes His Luck, Story:catalogue |
| MYT-26 | Sensing evil | A chill or dread warns dwarves that an evil presence is near (a vampire, a ghost, a lurking horror). | Missing | S | ★★ | N01 | Story:catalogue |
| MYT-27 | Cursing the gods | After disaster, some dwarves curse their god aloud and the devout are scandalised, starting grudges. | Missing | S | ★★ | C10 | Story:catalogue |
| MYT-28 | Temple design from the god | A god's spheres decide its temple's shape, materials, items, festivals and rites (open sky, black stone). | Partial: Architect's cross-shaped temple | S | ★★ | C10 | Bloat#382, Log2006-19, roadmap Gods with spheres |
| MYT-29 | Defilement | An atrocity at a sacred place corrupts it and its creatures, birthing a new evil site. | Missing | M | ★★★ | N03 | Story:Forest Befouled, Story:catalogue |
| MYT-30 | Places of power | A sacred spring heals the worthy, a grove shields its creatures: a site feature with rules. | Missing | M | ★★★ | N03 | Story:Forest Befouled, Story:catalogue |
| MYT-31 | Spirits of the land | A site lives in harmony with nature spirits or ancestor ghosts until the fort breaks it (felling, killing in the grove); elves are tied to them. | Missing | M | ★★ |  | 2020, PG64, PG135, Story:Root |
| MYT-32 | Prophecies | An engraving or tablet foretells an event (an eclipse, a beast's return) that may come true. | Missing | M | ★★★ | N03 | Story:The Reign of Cenaster, Story:Dwarven Servitude, Story:catalogue |
| MYT-33 | Demonic pacts | A desperate dwarf bargains with a dark power; help arrives at key moments, then the price is taken. | Missing | M | ★★★ | N03 | Story:The Evil Squire, Story:Dragon Quest, Story:catalogue |
| MYT-34 | Living statues | Statues get descriptions, can hide levers and moving parts, and rarely come to life. | Missing | M | ★★★ |  | Bloat#224 |
| MYT-35 | Automatons | Mechanical people, so rare only a dwarf in a fey mood can build one. | Missing | L | ★★★ |  | Bloat#42 |
| MYT-36 | Relics and heirlooms | Stored holy relics and family heirlooms awaken with powers. | Missing | M | ★★ | N03 | 2020 |
| MYT-37 | Curse framework | Curses as afflictions with a trigger and a lifting condition (full moon, sunlight, drinking blood, water), sometimes spreading through a group. | Missing | M | ★★ | N03 | Log2006-19, Story:Curse of the Cyclops, Story:Orther Pushes His Luck, Story:catalogue |
| MYT-38 | Holy orders against the dead | Faiths found orders that hunt undead and destroy dark lore; alliances form against necromancers and demons. | Missing | M | ★★ |  | Log2020-26, Log2006-19 (alliances) |
| MYT-39 | Pilgrims and monks | Faiths spread along trade routes; pilgrims visit the fort's temple, holy places grow. | Missing | M | ★★ |  | Log2006-19, PG86 |
| MYT-40 | Religious riots | Rival faiths clash; rulers tear down temples and expel believers; priests calm or stoke the riot. | Missing | M | ★★ |  | Log2006-19, PG88 |
| MYT-41 | Faith passed on | Children take their parents' gods some years after birth; immigrants keep theirs; the godless near a temple convert in waves. | Missing | S | ★ | C10 | Log2020-26, Log2006-19 |
| MYT-42 | Altars and offerings | Stone altars and offering places in a temple, where dwarves leave goods. | Missing | S | ★ |  | Log2020-26 |
| MYT-43 | Temple missions | High priests hand out open tasks: found a temple, recover a relic, deal with a rival faith. | Missing | L | ★★ |  | Bloat#377 |
| MYT-44 | Demons posing as gods | Demons and forgotten beasts take over goblin societies as false gods; a dark god can loose a demon. | Missing | M | ★★ |  | Log2006-19 (Rel0.31), Core89 |
| MYT-45 | A resident wizard | A wizard moves in; the fort helps their research (assistants, lab zone, errands), a dwarf can apprentice, and the wizard helps against magical sieges. | Planned: B14 | L | ★★★ | B14 | 2020, 2025 |
| MYT-46 | Wizards in the world | Wizards live alone, form covens or meddle in civilizations from towers with labs, libraries and summoning rooms themed by their art. | Planned: B14 | L | ★★★ | B14 | Arc Wizard, 2020, Log2020-26 (magical peoples and labs) |
| MYT-47 | Magical invasions | Sieges by magical peoples and summoned things that need magic or banes to answer. | Planned: B14 | L | ★★★ | B14 | 2025, B14 |
| MYT-48 | Divine powers and ceremonies | Temple ceremonies; gods grant powers that help defend the fort. | Planned: B14 | L | ★★★ | B14 | 2025 |
| MYT-49 | World faiths and cults | World-spanning religions with priestly posts (death, war), cults and schisms. | Planned: B09 sects, B14 | L | ★★ |  | Bloat#344, Core63, PG86 |
| MYT-50 | Corrupting infiltrators | Magical beings hide inside society; inquisitions hunt them. | Missing | L | ★★ |  | 2020 |
| MYT-51 | Lesser magic items | Wands, potions, ointments, runestones and rune-etched gear, with costs and rituals. | Missing | L | ★★ | B14 | 2020, Bloat#97, PG128 |
| MYT-52 | Magic with a bodily cost | Powers leave growing marks (a rotting finger, bark on the skin), and forbidden magic can kill its user. | Missing | L | ★★ |  | Story:Cado's Magical Journey, Story:Tales Foretold: Queens and Kings, Story:catalogue |
| MYT-53 | Dark moods and dark alchemy | A dwarf in a dark mood seizes a workshop and makes something from dwarven remains. | Partial: strange moods (psyche.rs), no dark kind | M | ★★ |  | Bloat#237 |
| MYT-54 | Telepathic bonds | A creature bound mind to mind with one keeper, who can pass the bond on; a bonded beast can carry word of a death home. | Missing | M | ★★ |  | Story:Dragon Rider, Story:Animal Justice, Story:catalogue |
| MYT-55 | Magic shapes the land | Magical regions (singing stones, bleeding trees) replace plain good and evil; some forts are built around one. | Partial: evil biome, blood rain | L | ★★ |  | Core94, 2020 |
| MYT-56 | Portals | One-way gates that bring creatures or strange materials into the world. | Missing | L | ★★ | B14 | 2020 |
| MYT-57 | Summoning and binding | A bound creature serves its summoner; a flawed binding lets others command it or gives it quirks. | Missing | L | ★ |  | Story:Moclem, Story:Warriors of the Dead, Story:catalogue |
| MYT-58 | Creation myths | Generated cosmologies explaining death, the afterlife, landforms and great beasts, with world settings from no magic to magic everywhere. | Planned: B14 | XL | ★★ | B14 | 2018-2020 myth, Arc World-gen Parameters, Bloat#134, Story:catalogue |
| MYT-59 | Taboo names | Some names are never spoken; saying one is defiance or a summons. | Missing | S | ★ |  | Story:catalogue |
| MYT-60 | Alchemy | Chemicals, acids and their reactions. | Missing | L | ★ |  | Bloat#269 |
| MYT-61 | Afterlife and planes | Dream, shadow, underworld and faerie realms where the dead go and can be visited. | Missing | XL | ★ |  | Story:Cado's Magical Journey, Story:Warriors of the Dead, Story:catalogue |

*Dropped: data-driven script generators for curses and divine items (Log2020-26, engine), wizard-tower play mode (Bloat#147, adventure/other mode).*

## Creatures & wildlife (CRE)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Species and bodies | Hundreds of creatures from data, each with its own body plan, materials and abilities | 29 species sharing one scaled 7-part body (`src/creature.rs`, `src/body.rs`) | No body variety; no flight, climbing, multi-head |
| Wildlife by biome | Each region has its own creatures in finite, recovering populations | Grazer, pack hunter and rare beast per biome (`combat.rs` `fauna`) | Not finite or recovering; wolves cause ~20% of deaths |
| Predator behaviour | Hunters stalk, flee when hurt, avoid strong groups; fear of fire | Hostiles chase within an aggro radius, otherwise prowl (`game.rs`, `combat.rs`) | No discretion, fear or stalking |
| Megabeasts | Titans and dragons from history attack wealthy forts, keep hoards, can take over a fort | Megabeasts from history after year 1; dragon fire, skull and hoard (`combat.rs` `spawn_megabeast`, `dragon_hoard`) | No lair trips, eggs, flying snatch, fort takeover |
| Forgotten beasts | Generated, unique: materials, breath, poison, webs, syndromes | One generated cave horror at a time, named and described but stat-identical (`combat.rs` `make_cave_horror`) | Description has no mechanical effect |
| Night trolls, kidnappers | Night creatures that carry off spouses and raise young in lairs | None | Missing |
| Goblin snatchers | Sneaks steal children who return as goblin soldiers | Skulkers steal the best loose item (`skulker_think`) | Children never taken |
| Kobolds | Thieving tricksters who flee, hoard and trap | Skulkers (generic thieves) | No kobold people |
| Gremlins and pests | Invisible pranksters on levers; vermin plague stores and fields; cats hunt them | Cats adopt dwarves (`src/game/pets.rs`) | No vermin, no gremlins |
| Monster hunters | Slayers petition to hunt cavern beasts and may die trying | None (F07 planned) | Missing |
| Animal people | Humanoid animals as visitors, invaders, citizens; guardians of forests | None | Missing |
| Scavengers | (Story/PG) carrion eaters gather at battlefields and refuse | Corpses rot into miasma (`src/game/miasma.rs`) | Missing |
| Beast legends | Man-eaters gain names; beasts in history return | Megabeasts are history figures; slain ones go into legends (`overworld.rs`) | Ordinary wildlife never becomes legendary |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| CRE-01 | Predator discretion | Wild hunters weigh the odds: they hold off from groups, soldiers and guards, and pick on the lone woodcutter instead. Directly targets the wolf deaths in the batches. | Missing | S | ★★ | S08 | Story:Root, Story:catalogue, Bloat#223 |
| CRE-02 | Fire and intimidation | Shouting, standing tall or waving a torch drives animals off; beasts once burned fear flame. | Missing | S | ★★ | S08 | Story:The Beast, Story:catalogue |
| CRE-03 | Wary once wounded | A hurt animal breaks off and becomes cautious, rather than fighting to the death. | Missing | S | ★ | S08 | Story:catalogue |
| CRE-04 | Baby snatchers | Goblin (or kobold) snatchers slip in at night and carry off a child, who may return years later as a goblin soldier; a rescue mission can follow. | Planned: Inbox (kidnappers); rescues B03 | M | ★★★ | B21 | Core89, Req275, Req517, Bloat#44, Log2020-26, Log2006-19 (thieves and kidnappers first), Story:Snatcher, Story:Night of the Kobold, Story:catalogue |
| CRE-05 | Forgotten beasts with powers | The cave horror's description becomes real: fire or poison breath, webs, spat globs, a body of crystal or iron, poison it is immune to, a statue of itself when slain. | Partial: generated name and look only (combat.rs) | M | ★★★ | B22 | Log2020-26, Log2006-19 (Rel0.31), Core93 |
| CRE-06 | Scavengers | Carrion birds, trolls and ghouls gather at battlefields and refuse piles; trolls drag bodies off, kobolds pick the pockets of the dead. | Missing | S | ★★★ | B21 | PG17, PG180, Story:Battlefield Lunch, Story:catalogue |
| CRE-07 | Monster hunters | Slayers and mercenary bands come to hunt a cave horror or night creature, may die trying, and a temple helps them with lore. | Planned: F07 | M | ★★★ | B22 | Night-creature role, Log2006-19 (monster slayers), Story:Monster Quest, Story:A Terror to Behold, Story:catalogue |
| CRE-08 | Man-eaters become legends | A wild beast that kills a hunter gets a name and a place in history, lairs nearby and comes back; it carries old wounds and a grudge. | Partial: megabeasts from history | S | ★★★ | B22 | Log2006-19 |
| CRE-09 | Disturbed lairs | Prodding a beast in its lair (an expedition, a mine) brings it to the fort in a rage. | Partial: expeditions hunt lairs (expeditions.rs) | S | ★★★ | B22 | Story:Dog-man, Story:catalogue |
| CRE-10 | Cyclic beasts | A legendary creature returns every so many years, and the legends warn the fort a season ahead. | Missing | S | ★★★ | B22 | Story:Dog-man, Story:catalogue |
| CRE-11 | Gremlins | Invisible pranksters that pull levers, open cages and spring traps. | Missing | S | ★★★ | B21 | Log2006-19 (Rel0.31) |
| CRE-12 | Proper kobolds | Small trickster thieves who flee rather than fight, set traps, keep poisonous critters, hoard shiny things and taunt their marks. | Partial: skulkers | M | ★★★ | B21 | Core90, 2025, Log2006-19 (kobold thieves, kobolds as pests), Story:Night of the Kobold, Story:Bride Price, Story:catalogue |
| CRE-13 | Flying snatchers | Rocs, giant eagles and dragons grab a dwarf and drop them from a height; flying matters to defence. | Missing | S | ★★★ | B22 | Core40, Req324, 2025, Story:Summoned to Darkness, Story:catalogue |
| CRE-14 | Night trolls | A unique night creature carries off a spouse, transforms them, raises young in a lair littered with its victims' gear. | Missing | M | ★★★ | B21 | Log2006-19, Log2020-26 |
| CRE-15 | Stalking predators | Predators shadow prey, wait for nightfall or a straggler, chase those who run and switch to easier prey. | Partial: hostiles chase in a radius | M | ★★ | S08 | Story:The Beast, Story:catalogue |
| CRE-16 | Savage wilds bite back | Heavy felling and industry anger the local wild; packs of enraged beasts storm the fort; elves anger sooner and hit harder. | Missing | M | ★★★ | X04 | Log2020-26 |
| CRE-17 | Vermin | Rats plague hoarded food and burrow into fields, eat seeds left in piles and can spread disease; cats and dogs earn their keep hunting them. | Missing | M | ★★★ | X04 | Bloat#11, Bloat#12, Bloat#18, Req336, Log2020-26 (vermin raids), Log2006-19 (vermin colonies) |
| CRE-18 | Dragon lairs and hoards | Dragons and giants carry off goods, livestock and drink over many trips to a lair with eggs and hatchlings, which can be raided. | Partial: dragons drop a hoard when slain | M | ★★★ | B22 | Log2006-19 (hoards, nests), Story:Mother of Death, Story:catalogue |
| CRE-19 | Megabeasts take fallen forts | A dragon or forgotten beast that destroys a fort moves in, leaving a ruin to reclaim later. | Missing | M | ★★★ | B22 | Log2006-19 |
| CRE-20 | Hot and cold creatures | Beings that radiate heat or cold, freezing water or scorching the air; melted by fire, quenched by water. | Missing | S | ★★ |  | Story:Cold Fire, Story:Mother of Death, Story:catalogue |
| CRE-21 | Hydras fight on | Each head attacks on its own; one head's blow does not stop it. | Missing (hydra exists as a megabeast) | S | ★★ | B22 | Log2006-19 |
| CRE-22 | Silent woods | Wildlife falls quiet and birds scatter when danger is near, a cue dwarves (and the overseer) can notice. | Missing | S | ★★ | S08 | Story:catalogue |
| CRE-23 | Wildlife from lairs | Lairs and camps send out groups that can be followed home, not random spawns. | Partial: prowling hostiles | M | ★★ | S08 | Log2006-19 |
| CRE-24 | Guardian animal people | Forest protectors that hunt anyone who kills within their wood. | Missing | M | ★★★ | X04 | Explorer role, PG135, Story:Root, Story:Animal Justice |
| CRE-25 | Generated megabeasts that breed and bargain | Megabeasts and lesser powers generated within limits (many kinds of dragon), able to breed and to parley for tribute. | Partial: history megabeasts; Planned: B02 (parley) | M | ★★★ | B22 | Core93, Better megabeasts 2012-2020, Story:catalogue (parley) |
| CRE-26 | Thieving animals | Magpies and monkeys steal items; one that takes an artifact becomes a history figure. | Missing | S | ★★ | B21 | Log2006-19 |
| CRE-27 | Creature antics | Animals root in mud, dig for shiny stones and call out instead of only wandering. | Missing | S | ★★ | X04 | Bloat#241 |
| CRE-28 | Minotaurs and labyrinths | Beasts living in mazes armed with their victims' weapons; their lair shows what they ate. | Missing | S | ★★ |  | Log2006-19 |
| CRE-29 | Living populations | Each region has a finite, believable population that falls with hunting and recovers; predators hunt prey. | Partial: wildlife by biome | M | ★★ | S08 | Core78, Req80, Req561, 2012, Bloat#285, Log2020-26 (finite wildlife), Log2006-19 (regrowth) |
| CRE-30 | Vision and stealth | Creatures see forward and to the sides; night and fog shorten sight; sneakers hide in grass. | Planned: B13 | M | ★★ |  | Log2006-19 |
| CRE-31 | Gentler cave life | Most cave animals behave like ordinary wildlife, not maniacs, and some could be kept. | Partial: cave beasts prowl the caves | S | ★★ |  | Bloat#169 |
| CRE-32 | Tracks, scent and climbing | Creatures climb and leap, leave tracks and smells; keen noses sense prey downwind; hunters follow spoor. | Missing | L | ★★ |  | Log2006-19, Story:Monster Quest, Story:The Beast, Story:catalogue |
| CRE-33 | Heat and cold harm | Extreme heat and cold injure creatures and plants; dwarves stay in during extreme weather. | Partial: blizzards chill | M | ★ |  | Log2006-19 |
| CRE-34 | Generated wildlife and bestiary | Vermin, beasts, soil creatures and plants generated and named, with a bestiary of what the fort has seen. | Missing | L | ★★ | X04 | Core94, Explorer role |
| CRE-35 | Animal people and extinct beasts | Humanoid animals as visitors, invaders and citizens; extinct animals (moas, dinosaurs) at a chosen rarity. | Missing | M | ★ |  | Log2020-26 (x2), Story:catalogue |
| CRE-36 | Swamp and shore life | Mosquitoes and leeches, lilies, gulls, crabs, jellyfish, driftwood and shells. | Missing | S | ★ |  | Bloat#290, Bloat#291, Bloat#328 |
| CRE-37 | Large shove small | Big creatures push smaller ones out of the way. | Missing | S | ★ |  | Story:catalogue |
| CRE-38 | Visible swimmers | Fish and salmon runs shown in rivers and ponds. | Partial: the legendary fish (fishlore.rs) | S | ★ |  | Bloat#81 |
| CRE-39 | Snakes and worms | Legless creatures that keep to the ground. | Partial: vipers | S | ★ |  | Bloat#191 |
| CRE-40 | Generated peoples | Whole civilizations of generated creatures with their own items. | Missing | XL | ★ |  | Core95 |

*Dropped: vermin bodies built only on demand (Bloat#362, engine).*

## Digging, caverns & underworld (DIG)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Mining | Speed depends on pick material, rock hardness and skill; priorities; vein auto-follow | Dig designations and an exclusive Mining labor (`src/world.rs` Desig, `labor.rs`) | Flat speed, no vein following |
| Prospecting | Finds announced; ore shown through smoothed walls; embark lists ores, flux, clay | Ore veins and gems in the rock, site features at embark (`src/terrain.rs`) | No announcements or prospecting |
| Cavern layers | Three layers, each named, with its own lakes, fungus forests, creatures and peoples | 2-3 layers with pools and tower mushrooms; spiders and troglodytes per layer (`terrain.rs`, `combat.rs` `populate_caverns`) | No fungus wood, no cavern peoples, no names |
| Reveal | Caverns revealed gradually as dwarves come near | Flood-fill reveal on dig (`game.rs`) | Close; no "unknown" restriction |
| Irritation | Mining, logging and dumping in the caverns anger what lives there | Cave horror once caverns found (`combat.rs` `maybe_cave_horror`) | No link to the fort's activity |
| Breach defence | Players wall or door off breaches | Overseer doors every breach at once (`overseer.rs`) | Done; DF is no deeper |
| Underworld | A final layer and a rare deep metal; breaching or over-mining it brings demons | None; bedrock under the magma sea | Missing entirely |
| Magma | Magma sea with its creatures | Magma sea flows, burns, sets to obsidian; magma forges (`game.rs` fluids) | No magma creatures (W26) |
| Cave-ins | Unsupported ground collapses; rubble injures | Wide spans collapse with a creak warning; always deadly (`src/game/disasters.rs`) | No disconnection rule, no survivors |
| Darkness | Light and dark planned; caves feel dangerous | No lighting (B13 planned) | Missing |
| Chasms | Bottomless chasms for disposal; their dwellers can be provoked | Chasm site feature (`terrain.rs`) | No dumping into it |
| Deep sites | Deep-dwarf towns, tunnels between sites, kobold warrens, goblin pits | Sealed ancient vault with warden (`src/game/ruins.rs`); tappers of the deep roads carry news (`word.rs`) | No other underground sites |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| DIG-01 | Cavern irritation | Noise from mining, logging fungus and dumping refuse into chasms or rivers builds an irritation score that brings cave horrors and underground peoples up. | Partial: a cave horror once caverns are found | S | ★★★ | D01 | Log2020-26, Log2006-19 (noise irritates the depths), Req8 |
| DIG-02 | Warnings from below | Before the deepest horror is reached the caves change: tremors, warm draughts, fleeing cave life, dwarves' nightmares; afterwards the caves stay changed. | Missing | M | ★★★ | D02 | Bloat#69, Bloat#23, Core12, Core16, Core39 |
| DIG-03 | Cavern peoples | Each layer has its own folk that raid from below and keep coming while an edge is open: olm-men from the water, bat riders with blowguns, ant-folk with queens and nests. | Partial: troglodytes per layer | M | ★★★ | D01 | Req232, 2025, Log2006-19 (underground invasions), Log2020-26 (underground peoples), Story:Summoned to Darkness, Story:catalogue |
| DIG-04 | The underworld | Below the magma sea lies a sealed layer of generated demons; breaching it sends them through the fort, and a fort can be destroyed from within. | Missing | L | ★★★ | D02 | Log2006-19 (hidden fun stuff, Rel0.31), Log2020-26, Core12, Core16, Core39, Bloat#23, Story:Summoned to Darkness, Story:catalogue |
| DIG-05 | Deep metal greed | A rare, priceless deep metal in hollow tubes; mining past a set amount wakes what lies below, a classic overseer temptation by its Greedy trait. | Missing | M | ★★★ | D02 | Log2006-19 |
| DIG-06 | Dig speed by pick and rock | Mining speed depends on pick metal, rock hardness and skill (a legend twice a novice); felling on axe metal. | Missing | S | ★★ | O10 | Log2020-26, Bloat#282, roadmap Mineral properties |
| DIG-07 | Surviving cave-ins | Falling rubble injures and buries instead of always killing; others dig the trapped out. | Partial: cave-ins with a creak warning (disasters.rs) | S | ★★ | O10 | Bloat#110 |
| DIG-08 | Dark caverns | Caverns are dark; creatures there go unseen until close; torches and lamps push the dark back; dwarves dislike working in it. | Planned: B13 | M | ★★★ | D01 | Req96, Core55, Arc Fire and Lighting, Story:catalogue (darkness and blindness) |
| DIG-09 | Sounds from the deep | Distant events reach the fort as sounds ("screams from the lower caverns") rather than full reports, feeding fear and the overseer's Danger sense. | Missing | S | ★★ | D01 | Story:catalogue |
| DIG-10 | Rich cavern soil | Surface soil far above the caverns farms poorly, cavern soil richly, pulling farms (and dwarves) into dangerous ground. | Partial: crops by place | S | ★★★ | D03 | Log2020-26 |
| DIG-11 | Follow the vein | A vein designation spreads along the ore or gem cluster as it's uncovered, with ore-only or gem-only filters and minimum skills. | Missing | S | ★★ | O10 | Bloat#218, Req393, 2016, Log2006-19 |
| DIG-12 | Prospecting and finds | Breaking into a vein or gem cluster is announced; smoothed walls show ore behind them; embark lists ores, flux and clay. | Partial: site features at embark | S | ★★ | O10 | Log2006-19, Inbox (prospecting) |
| DIG-13 | Endless cavern life | Cave creatures arrive from a large off-map pool while a cavern edge stays open, so a breach is a lasting choice. | Partial: one-time cavern population | S | ★★ | D01 | Log2006-19 |
| DIG-14 | Webs | Giant spiders web their victims and ceilings and walk their webs over open air. | Partial: cavern spiders | S | ★★ | D01 | Bloat#345, Story:Summoned to Darkness, Story:catalogue |
| DIG-15 | Cave fliers | Swarms of flying cave creatures in patterns; bats over the chasm. | Missing | S | ★★ | D01 | Bloat#332 |
| DIG-16 | Cave side stories | Small scripted scenes found in the caves: a lost miner's camp, a trapped beast, a shrine. | Partial: mysteries (F16), ruins (F08) | M | ★★★ | X01 | Bloat#26 |
| DIG-17 | Gradual reveal and the unknown | Caves reveal as dwarves come near, and squads can't be ordered into unseen ground. | Partial: flood-fill reveal | S | ★★ | D01 | Log2006-19 |
| DIG-18 | Fungus forests | Tower-cap forests give underground wood and spread their spores into dug-out cave soil once a cavern is opened. | Partial: tower mushrooms as scenery | M | ★★ | D03 | Core91, Bloat#189, Bloat#304, Log2006-19 (spore spread, giant mushrooms), Inbox (cavern fungus forest) |
| DIG-19 | Chasm disposal | Refuse, corpses or unwanted goods thrown into a bottomless chasm (and what lives below notices). | Partial: chasm site feature, garbage dump zone | S | ★★ | D03 | Req28, Req47, Log2006-19 |
| DIG-20 | Magma dwellers | Creatures living in the magma sea that rise when it's tapped. | Planned: W26 | S | ★★ | D03 | Req355 |
| DIG-21 | Warm and damp stone | Digging near magma or water warns first; a standing order can ignore it. | Partial: overseer avoids aquifers and lakes | S | ★★ | O10 | Log2020-26 |
| DIG-22 | Unsupported collapse | Ground no longer connected to support falls, and undercut ramps and stairs give way. | Partial: span-based cave-ins | M | ★★★ |  | Log2006-19, Req459, Req506, Req507 |
| DIG-23 | Crypts of heroes | Catacombs where the fort's heroes lie, each tomb carved with their deeds; a place dwarves visit. | Partial: tombs, coffins, memorials (family.rs) | S | ★★ | X01 | Story:Dragon Quest, Story:catalogue |
| DIG-24 | Ruins tell how they fell | A ruin's state shows whether it was besieged from outside or gutted from within, and what did it. | Partial: ruins with legend engravings (ruins.rs) | S | ★★ | X01 | Story:catalogue |
| DIG-25 | Kobold warrens | Trap-filled kobold caves with hoards to raid, or to fear when they raid you. | Missing | M | ★★ | X01 | Core90 |
| DIG-26 | Underground waters | Lakes, rivers and waterfalls between cavern layers; platforms on lakes; crossing by raft. | Partial: cavern pools, underground-river site feature | M | ★★ | D03 | Bloat#190, 2011 Maps I.A, Story:The Adventures of Gornon, Story:catalogue |
| DIG-27 | Deeper, stranger layers | A procedural, more magical underground with stranger creatures the deeper you go. | Partial: 2-3 layers | L | ★★ | D02 | 2025, Inbox (deeper caverns) |
| DIG-28 | Digging too deep in history | In world generation, dwarves dig too deep and unleash disasters that spread to other sites, including yours. | Missing | M | ★★ | D02 | Log2020-26, Log2006-19 |
| DIG-29 | Deep sites and tunnels | Deep-dwarf towns, sites under ranges and tunnels between forts for underground trade, full of beasts. | Partial: deep-road tappers carry news | L | ★★ | X01 | Bloat#172, Core28, Core29, Log2006-19 |
| DIG-30 | Goblin pits | Tunnel networks and death pits under goblin towers. | Missing | M | ★ | X01 | Bloat#383, PG100 |
| DIG-31 | Fossils | Fossils and petrified wood dug from the rock, wanted by collectors. | Missing | S | ★★ |  | Bloat#324 |
| DIG-32 | Pits and leaps | Open pits in the caves to fall into or jump across. | Missing | S | ★ |  | Bloat#227 |
| DIG-33 | Gems in their host rock | Each gem appears only in its proper host rock. | Partial: one gem kind (amethyst) | S | ★ | O10 | Bloat#64 |
| DIG-34 | Digging out boulders | Boulders embedded in the ground can be quarried. | Missing | S | ★ | O10 | Bloat#340 |
| DIG-35 | Mining options | Dig without leaving stone behind; climb out of a pit you dug yourself into. | Missing | S | ★ |  | Req74, Req552 |
| DIG-36 | Dark-vision draughts | A brew lets dwarves see in the dark but makes their eyes glow. | Missing | S | ★ |  | Story:Orther Pushes His Luck, Story:catalogue |
| DIG-37 | Sunken dungeons | Flooded dungeons, ice grottos and caves full of stranger liquids. | Missing | M | ★ | X01 | Bloat#17 |

*Dropped: Have rows (veins at every depth Bloat#240, better-looking stone Bloat#65); claiming any cave with an adventurer band (Bloat#182, adventure); underwater swimming and diving for adventurers (Bloat#17, adventure part only).*

