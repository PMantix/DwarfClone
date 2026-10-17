# War: military, sieges and invaders

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## Military & combat (MIL)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Wound model | A blow passes through armour, then skin, fat, muscle and bone. Each material's density and toughness decides whether it cuts, bruises or breaks, and repeated blows to one spot add up | 7 body parts with hp and 5 wound grades (bruise to fatal), severing, limping, unconsciousness; edged vs blunt (`body.rs`, `game/combat.rs`) | No layers or material physics, and armour is either worn or not |
| Bleeding and lethality | Bleeding is set per wound. Fights end decisively through openings on a downed foe, and the wounded can be dressed and carried off | A blood pool drained at a bleed rate that grows with the square of the wounds; 59-62% of all deaths | The curve needs reshaping (S02). Nobody presses or bandages wounds |
| Skills and attributes | Separate skill for each weapon, about 19 attributes that grow with use and rust without it, teaching, a cap on what one exchange can teach | One `fight_xp` combat skill; a militia commander speeds training | No attributes, rust or per-weapon skill (W20) |
| Weapons and materials | Many weapon kinds; material and quality set how well they work; named and favoured weapons | Pick, axe, sword, spear, hammer, crossbow (wood only) and bolts, in copper, bronze, iron, steel or wood (`items.rs`) | No metal crossbows. Soldiers club with wooden ones |
| Ranged combat | Bow vs crossbow, loading ahead then a quick shot, aimed shots, accuracy falls with range, archers back off and use fortifications, squad ammo pools split by material | Marksdwarves shoot up to 10 tiles with a clear line, through fortifications; a quiver of bolts (`game/military.rs`) | No load state, aiming, retreat or ammo pool. Hardly any bolts get fired |
| Defence and armour | Parry, block or dodge; one weapon can't parry a crowd; armour wears and passes some force through; helms get pulled off the fallen | Shield blocks 25%; helm, breastplate, leather | No active defence, no wear. Armour sits unworn in stock |
| Squads and orders | Monthly routines, named alerts, patrol routes, burrow defence, replacements step in when one soldier leaves | Named squads with train, station or kill orders; drafting; one civilian alert (`game/military.rs`, burrows) | No schedules, patrols or burrow orders (Inbox) |
| Uniforms and gear | Uniforms list item kinds and materials; separate training, battle and civilian kit; a readiness grid; racks; a quartermaster | Weapon type (Best/Axe/…) and armour on or off per squad | Too coarse; readiness can't be seen |
| Training | Barracks sparring, demonstrations by skilled soldiers, injuries from careless sparring, archery lanes that teach better at longer range, training weapons | Barracks sparring, archery targets, commander bonus | No teaching, safety rules or range rules |
| Morale and fear | Fear when attacked, poor courage leads to fleeing or rage, levels of conflict, yielding, an army breaks when its leader falls | Civilians flee; duels end in a yield; an army routs when its general dies (`game/combat.rs` `rout`, `game/drama.rs`) | Dwarf soldiers never break, rally or surrender |
| Grappling | Grabs, throws, chokes, reversals, dragging, crushing at the bottom of a pile-up, big creatures tearing smaller ones apart | Arm-wrestling at festivals only | Missing entirely |
| Civilians in a fight | Bystanders join or flee depending on friendships and personality; cornered folk grab whatever's at hand | Civilians run to the shelter, and 32% of the dead died there (baseline) | Civilians never fight back |
| Animals and mounts | War and hunting animals, mounts that carry speed into a charge, trained beasts | War dogs follow a soldier (`game/animals.rs`) | No mounts, no beasts beyond dogs |
| Armies off the map | Raiding vs razing, conquest leading to a barony, tribute, rescues, artifact recovery, world-screen missions | F09 expeditions to a ruin, an enemy site or a lair, with odds from strength alone (`game/expeditions.rs`) | No captives, no conquest or tribute, nobody comes back for revenge (B03, S06) |
| Records and legends | Kill lists for each figure, weapons that earn names, battles named in the legends, combat logs | Epithets earned from kills (Grim, Red-Handed, Xbane…, `game/heroes.rs`); siege notes; a general's death in the legends | No named weapons or named battles; no readable fight log in play |
| The overseer as commander | (The player does it in DF) | Drafts soldiers, stations them at the lever, arms the best first, raises the bridge (`game/overseer.rs`) | Never orders a kill (wolves cause 20% of deaths); doesn't get armour onto soldiers before the first siege |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| MIL-01 | Metal crossbows and bolts | The forge makes iron, bronze and steel crossbows and bolts. Wooden ones are for practice, and a marksdwarf never clubs with a crossbow while melee weapons are in stock | Planned (Inbox: metal crossbows & bolts) | S | ★★★ | S07 | WL (wooden crossbows, 92 bolts in 31 sieges); Log2025; Inbox |
| MIL-02 | Kit on before the first siege | A readiness check per soldier and kit slot: has it, assigned but not collected, none to be had. The overseer reads a shortfall as a trouble and fetches armour from stock or orders it | Partial: uniforms, "Unready" sense | S | ★★★ | S07 | Log2021 (equipment grid); WL (10-32 armour pieces unworn) |
| MIL-03 | The overseer hunts predators | When wolves, bears or a cave beast prowl near the fort, the overseer sends an armed squad with a kill order instead of letting haulers die one at a time | Partial: kill order exists for the player only | S | ★★★ | S07 | Log2009 (kill orders); Batch (wolves 18-22% of deaths) |
| MIL-04 | Wounds that leave time to help | Reshape blood loss so a cut limb kills in minutes and a gash in hours. Blows land on a specific part, and a downed foe gives openings so fights end cleanly instead of in a slow bleed | Planned (S02 step 4) | S | ★★★ | S07 | WL S02 note; Log2010 (tissue layers); Log2014 (openings); Batch |
| MIL-05 | Archers who act like archers | Ranged soldiers keep their distance, duck behind fortifications, refill from the squad's ammo instead of charging, and melee only when cornered. Enemy archers do the same | Partial: line of fire through fortifications | M | ★★★ | S07 | Log2025; Req76 |
| MIL-06 | Morale, fear and rout | Every fighter weighs wounds, losses and odds against courage. The timid break and run, some go berserk, a beaten foe yields, and a frightened shooter's hands shake | Partial: an army breaks when its general dies | M | ★★★ | A02 | Hero role 2016/2020; Bloat207; Log2014-15 (fear); Story:Mil#1; Story:Mind#14 |
| MIL-07 | Emergency levy and martial law | When the squad is overwhelmed, the overseer hands weapons to cooks and haulers for a last stand. Nobles may impose martial law, which everyone resents | Missing | S | ★★★ | A02 | Req273; Bloat236; Story:Mil#18 |
| MIL-08 | Cornered civilians fight back | A dwarf who can't flee grabs the nearest dangerous thing (pick, tongs, a chair leg, a burning log) instead of dying bare-handed in the shelter | Missing | S | ★★★ | A02 | Story:Mil#5; WL (32% died in the shelter) |
| MIL-09 | A leader's fall cuts both ways | When a commander dies, their followers either lose heart or go into a short vengeful frenzy, depending on how loyal they were | Partial: rout only | S | ★★★ | A02 | Story:Mil#2; Log2014 (leaderless armies) |
| MIL-10 | Rallying charge | A brave dwarf who charges alone can turn a fleeing squad back to the fight, and the chronicle records it as a deed | Missing | S | ★★★ | A02 | Story:Mil#3 |
| MIL-11 | Weapons that earn names | A weapon keeps a tally of its kills. Enough kills, or one important one, earn it a name like a minor artifact, and its owner grows attached (marks, initials, quarrels over it) | Missing | S | ★★★ | A03 | Log2009/2014; PG133; Core99; PG169-170; Story:Crafts#1 |
| MIL-12 | Named battles and a fight log | A readable log of each fight with war cries and taunts. Big clashes are grouped and named afterwards ("the Raid of …") in the legends and chronicle | Partial: siege notes, blow counts, generals in legends | M | ★★★ | A03 | Bloat270; Bloat244; Bloat219; Rel0.31 |
| MIL-13 | Revenge and nemesis | A dwarf who lost kin or limb comes to hate that kind of creature, seeks out the particular killer on the field, or swears a vow that steers their choices. The hatred dulls their pain | Missing | S | ★★★ | A03 | Bloat275; Story:Mil#4; Story:Mind#7 |
| MIL-14 | Mercenaries for board | Passing sellswords offer to join the military in return for food and a bed, and keep their own kit. Fewer overseers end up with no soldiers at all | Partial: wandering warriors may stay | S | ★★★ |  | Log2015; Rel0.42; 2016 taverns |
| MIL-15 | Mutiny and desertion | Soldiers who are miserable, starving or losing grumble, refuse orders, fight among themselves or slip away. A doomed army can melt away on the march | Missing | M | ★★★ | A02 | Bloat90; PG130; PG179; Story:Mil#20 |
| MIL-16 | Yielding and prisoners of war | A beaten attacker can plead for quarter (or fake it). The victor's personality decides whether to spare them. The spared become captives to ransom, execute or release | Planned (S03 captives) | M | ★★★ | A02 | 2020 Hero role; Story:Mil#6 |
| MIL-17 | Bystanders choose sides | Onlookers join a fight or flee depending on friendships, grudges and nerve, and they hold back when the berserker is a friend | Missing | M | ★★★ | A02 | Log2012/2014; Story:Society (war buddies) |
| MIL-18 | Armies abroad: raid, raze, loot | Squads (later whole armies) march off-map to raid, pillage or raze. Raiding and razing are different missions, and you choose what loot comes home | Partial: F09; Planned B03 | L | ★★★ | B03 | Core27; Core35; 2011 III.B; 2020; Log2014; Log2023 (world-screen missions) |
| MIL-19 | Rescue missions | A squad sets out after kidnapped dwarves or raiders captured on a failed mission, and brings them home or dies trying | Planned (B03 freeing prisoners) | M | ★★★ | B03 | Bloat44; Log2014 (captured raiders); Story:Adv#1 |
| MIL-20 | Recover stolen treasures | Send a squad after a stolen artifact. It comes back with the artifact, or a tale and rumours of where it went | Planned (B05) | M | ★★★ | B03 | 2020 |
| MIL-21 | Strike at enemy leaders | A mission to capture or assassinate a rival's position-holder, which shakes up their politics and their next siege | Missing (near B03) | M | ★★★ | B03 | 2020 |
| MIL-22 | The crown's levy | The monarch demands soldiers for the realm's wars. They return as veterans or never. The overseer has to choose between obeying and holding its gate | Missing | M | ★★★ | B03 | Core33 |
| MIL-23 | Field armies as a shield | Squads posted outside delay or turn back an approaching siege and send word ahead | Planned (B01) | M | ★★★ | B03 | 2020; 2025 |
| MIL-24 | Hostage shields | An attacker grabs a child or civilian to block a soldier's blow or to bargain | Missing | M | ★★★ |  | Story:Mil#16 |
| MIL-25 | Tear apart | Something far bigger simply rips a grappled foe to pieces | Missing | M | ★★★ |  | Bloat254 |
| MIL-26 | Squad routines and alerts | Named routines (train, ready, off duty) and fort-wide alerts that switch every squad's schedule at once. Someone fills in when a soldier leaves to eat. Soldiers don't stand around a commander while he eats or lies wounded | Planned (Inbox: squad schedules) | M | ★★ | A04 | Log2009; Log2021; Req120; Req545; Arc Army |
| MIL-27 | Patrol routes | Waypoints chained into a named, reusable route that patrollers walk exactly | Planned (Inbox: patrol routes) | S | ★★ | A04 | Log2010; Log2021 |
| MIL-28 | Defend a burrow | Soldiers assigned to a burrow drop everything when an enemy sets foot in it | Partial: station by the lever | S | ★★ | A04 | Log2010 |
| MIL-29 | Gear off duty | Soldiers rack their armour while relaxing, so an alarm during a festival catches them half-dressed | Missing | S | ★★ | A04 | Story:Mil#23; Arc Army |
| MIL-30 | Squad ammo pools | Each squad has ammo set by material, count and use (war, practice or both), kept in barracks chests. Metal bolts are saved for battle and bone or wood used for practice | Partial: a quiver per crossbow holder | S | ★★ | A04 | Bloat300; Log2009; Log2025 |
| MIL-31 | Load, then loose | Loading takes most of the time and can be done before the shot. The shot itself is quick, with a short recovery after | Partial: crossbows, no load state | S | ★★ |  | Log2021; Log2025 |
| MIL-32 | Aimed blows and gaps in armour | Fighters and shooters pick a body part (hard to reach a giant's head, easy on a biting dragon). Hits get harder with range. Veterans aim at unarmoured gaps and teach the squad an enemy's weak spot | Partial: wounds land per part | M | ★★ |  | Req230; Combat flow; Log2014; Log2025; Story:Mil#13 |
| MIL-33 | Friendly fire | A bolt aimed at a foe grappling a friend can hit the friend. An ally standing much closer to the shooter is safe | Partial: line-of-fire check, allies never hit | S | ★★ |  | Bloat222 |
| MIL-34 | Active defence | The defender chooses to parry, block or dodge. One blade can't parry many attackers, so a mob brings down a master | Partial: shield blocks 25% | M | ★★ |  | Log2013-14 |
| MIL-35 | Armour wear and jarring | Blocking a blow wears the armour and the weapon down. Blocked force still jars necks and joints. A victor pulls the helm off a downed foe. Grime on gear needs cleaning | Missing | M | ★★ |  | Log2014; Req149; Req431 |
| MIL-36 | Shoddy gear fails | Badly made weapons and crossbows misfire or snap. A settlers' kit made by unskilled hands may be shoddy | Missing | S | ★★ |  | Req34-35 |
| MIL-37 | Attributes and skill rust | Strength, agility and the rest grow with use and slowly fade without it. Related skills help each other | Planned (W20) | M | ★★ | W20 | Core72; Core73; Req32; Rel0.31 |
| MIL-38 | Teaching and drills | A soldier with more skill teaches recruits by demonstration, with no blows exchanged. Lazy students learn slowly. Reckless instructors rush recruits into live sparring | Planned (W20 teaching) | M | ★★ | A04 | Log2009; Log2019; Rel0.47 |
| MIL-39 | Sparring accidents | How hard and how often partners swing depends on personality: careful pairs rarely land a blow, reckless pairs hurt each other | Missing | S | ★★ | A04 | Log2014-15 |
| MIL-40 | Sergeant's discipline | A hard officer shames, beats or inspires shaky recruits into holding the line | Partial: militia commander | M | ★★ | A04 | Story:Mil#17 |
| MIL-41 | Favourite weapons | About a third of dwarves favour a weapon type. Familiar gear rolls better, and an attached veteran grumbles or refuses when re-equipped | Missing | S | ★★ | A03 | Req427; Log2019 |
| MIL-42 | Know your monster | Fighting or studying a kind of creature gives a lasting bonus against that kind | Missing | S | ★★ |  | Bloat76 |
| MIL-43 | Spoils of the field | After a siege, soldiers may take up the bows and whips the enemy dropped, with some penalty or grumbling. Standing orders decide what happens to captured gear and worn clothing | Missing | S | ★★ |  | Log2010s; Req436 |
| MIL-44 | One appointed champion | Legendary skill doesn't make everyone a champion. A noble appoints a single champion, which gives the fort a reason to keep that noble | Missing | S | ★★ | A03 | Log2009/2012 |
| MIL-45 | Military tactics skill | The best tactician (together with leadership) tips the odds of off-map battles, is trained by fighting, and is named in the mission report | Missing (expedition odds use strength only) | S | ★★ | B03 | Log2013; Rel0.47 |
| MIL-46 | Surprise and awareness | A separate skill decides who reacts to an ambush in time. Blows from outside a victim's field of view hit much harder | Missing | M | ★★ |  | Log2009; Rel0.40 |
| MIL-47 | Levels of conflict | Fights carry a level: horseplay, brawl, lethal, no quarter. Brawls avoid the throat and end in a yield. Insults climb step by step before blades come out. A safety valve calls off lethal feuds between residents | Partial: duels to a yield, tantrum punches | M | ★★ |  | Bloat207; Log2014; Log2022; Log2025 |
| MIL-48 | Battle noise raises the alarm | Clashes, screams and war cries carry some distance and wake sleepers, so the alarm goes up before anyone sees the enemy | Missing | S | ★★ |  | Story:Mil#9; Bloat88 |
| MIL-49 | Playing dead | A downed or terrified creature lies still among the bodies to escape notice, or to spring up later | Missing | S | ★★ |  | Story:Mil#7 |
| MIL-50 | Mercy kill | A soldier ends a mortally wounded friend's suffering. Others remember it as kind or monstrous | Missing | S | ★★ |  | Story:Mil#8 |
| MIL-51 | Dirty fighting | Sand or spit in the eyes, fighting with the sun at your back, using a dying foe as a shield, throwing powders that hang as a brief cloud | Missing | S | ★★ |  | Bloat15; Bloat162; PG77; PG96; PG150 |
| MIL-52 | Improvised ammunition | When bolts run out, crossbows and engines fire whatever odd objects are at hand | Missing | S | ★★ |  | Bloat359 |
| MIL-53 | Thrown flasks | Glass globes of acid, poison or burning oil, thrown or launched, that shatter on impact | Missing | M | ★★ |  | Story:Mil#21 |
| MIL-54 | Fire arrows and tar | Pitch-soaked targets and burning bolts set rafts, bridges and wooden gates alight | Partial: fire spreads | S | ★★ |  | Story:Mil#22 |
| MIL-55 | Bites that hold on | A biter that has latched on shakes to tear flesh and keeps pumping venom | Missing | M | ★★ |  | Bloat138 |
| MIL-56 | Accidental hangings | A creature chained by the neck that goes over a ledge strangles | Missing | S | ★★ |  | Bloat255 |
| MIL-57 | War beasts | Beyond dogs: trained big cats, beak-dogs, armoured animals, beasts pulling carts. War animals stay at heel while the squad has orders and are never butchered | Partial: war dogs | M | ★★ |  | Story:Agri#1; Log2025; Log2023 |
| MIL-58 | Saved uniforms by kind | A uniform names item kinds and materials, is saved by name and applied to many soldiers. Training, battle and civilian kit swap on schedule. Re-equipping prefers undamaged pieces | Partial: weapon type plus armour on or off | M | ★★ | A04 | Log2009; Log2021 |
| MIL-59 | Pillage depletion | Each raider brings home one item or animal. A raided site runs dry and recovers slowly | Partial: expeditions bring loot | S | ★★ |  | Log2014 |
| MIL-60 | Leaders away or captured | A commander away on a raid can't form squads. A captured leader forces a succession | Missing | S | ★★ |  | Log2014 |
| MIL-61 | Conquest, tribute, barony | A squad wins over or conquers an outside site, installs an administrator who sends yearly tribute, and the fort rises to a barony | Planned (B03) | L | ★★ | B03 | Core28; 2018; 2020; Log2014 |
| MIL-62 | Loyalty in the party | A party follows its leader but refuses cruel or senseless orders, depending on loyalty, and doesn't turn on itself in an ambush | Partial: squads on expeditions | M | ★★ |  | Core24; Core61; Hero role; Log2024 |
| MIL-63 | Army-vs-army battles | Armies that meet on the world map fight, take captives and flip towns. Large armies form lines across several cells | Planned (B01/B03) | L | ★★ |  | Core46; 2016; 2020 |
| MIL-64 | Fighting styles and schools | Named styles with their own moves, learned from respected masters. At the very top, a fighter can invent a new style and found a school | Missing | L | ★★ |  | Combat styles 2012-2020; PG171 |
| MIL-65 | Mounted combat | Mounts carry speed and momentum into blows and trample. Riders can climb onto huge foes | Missing | L | ★★ |  | 2012 R7; Mounts; Bloat289 |
| MIL-66 | Tissue layers and material physics | Damage is resolved layer by layer using material properties, so small creatures can scratch big ones and blunt force crushes through plate | Partial: body-part wounds, severing | XL | ★★ |  | Log2008-10; Rel0.31 |
| MIL-67 | Per-dwarf kill records | Every figure keeps a list of what they killed, where, and with named victims, shown on the unit sheet and in the legends | Partial: kill counts drive epithets | S | ★★ | A03 | Rel0.40; Log2010s |
| MIL-68 | Ammo sense | Soldiers pick up spent bolts and top up their quivers before running dry | Partial: quiver of bolts | S | ★ |  | Bloat102; Req134 |
| MIL-69 | Bows vs crossbows | A bow hits harder the stronger and more skilled the archer (to a cap). A crossbow loads slowly but hits full force for anyone. Skill speeds loading for both | Missing | S | ★ |  | Log2025 |
| MIL-70 | Training weapons | Blunt practice weapons from the carpenter make sparring far safer | Missing | S | ★ |  | Log2009 |
| MIL-71 | Capped skill gain | No single exchange teaches too much, and kills by traps or bolts don't train general fighting | Missing | S | ★ |  | Log2010s |
| MIL-72 | Archery range lanes | A range zone works out its firing direction from the targets and gives each trainee a lane. Longer shots and a real target teach faster | Partial: archery targets | S | ★ |  | Log2025 |
| MIL-73 | Racks and barracks roles | Each soldier claims a weapon rack and an armour stand. A barracks is marked for sleeping, training or storage, and squads are assigned to barracks | Partial: one barracks zone | S | ★ |  | Req18; Req413-414; Req449; Bloat353 |
| MIL-74 | Field rations and care packages | Settings per squad for food in the pack and booze (or water) in the flask. Comforts can be sent to dwarves away at war | Missing | S | ★ |  | Req417; Bloat45; Bloat175; Log2020 |
| MIL-75 | Hunter ammo cap | Hunters keep only a few bolts, so soldiers don't run short | Missing | S | ★ |  | Log2025 |
| MIL-76 | Drop the load on call-up | A dwarf called up while hauling drops the item instead of carrying it into battle | Partial | S | ★ |  | Log2021 |
| MIL-77 | Quartermaster | In a big fort, an appointed official issues gear and arranges upgrades | Missing (the overseer arms squads itself) | S | ★ |  | Log2009 |
| MIL-78 | Stuck and barbed weapons | A weapon lodged in a body hampers both fighters until the wielder lets go. Barbed blades worsen bleeding until they're cut out | Missing | S | ★ |  | Bloat137; Log2025; Story:Mil#12 |
| MIL-79 | Reach and set spears | Long weapons strike first and keep foes back. A spear set against a charge impales the charger | Missing | S | ★ |  | 2025; PG157; Story:Mil#11 |
| MIL-80 | Wrestling | Grabs, trips, throws, chokes, disarms, reversals, tangled limbs, dragging a foe over a ledge, crushing whoever is at the bottom of a pile-up | Partial: festival arm-wrestling | L | ★ |  | Core1; Req206; Bloat135-142; Bloat271; Story:Mil#10 |
| MIL-81 | Strike and move timing | Separate timings for striking and moving, with inertia, turning, and charges that knock foes back | Missing | M | ★ |  | Req188; Req330; 2012 R7 |
| MIL-82 | Stances and counters | Each exchange rolls fresh odds per move. Guard stances bring bonuses and costs. Some moves work only as counters | Missing | M | ★ |  | Req331; Combat flow 2012-2020 |
| MIL-83 | Formations and hesitation | Groups form up and advance together instead of trickling in. Cautious fighters hesitate before closing | Missing | M | ★ |  | Story:Mil#14 |
| MIL-84 | Balanced hand loadouts | Carried and wielded items are spread sensibly across both hands | Missing | S | ★ |  | Bloat129 |
| MIL-85 | Messages on missiles | Taunts carved on bolts or notes left for the enemy, shown in reports | Missing | S | ★ |  | Story:Mil#24 |
| MIL-86 | Cruel play | A much stronger foe toys with a victim for show instead of finishing it | Missing | S | ★ |  | Story:Mil#25 |

*Dropped as adventure-mode or engine trivia: the wrestling target-picking screen (Bloat140), the tactical party-control mode (Log2024), and squads called up from the main screen (Bloat154, already Have).*

## Sieges & invaders (SIE)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Where attackers come from | Real groups drawn from an enemy site's population march across the world map. Every kill weakens that site | An army appears at the map edge on the first day of a season, in a size set by formula (`game/combat.rs` `lay_siege`, `siege_strength`) | No march, and no link to any site's population (B01) |
| What triggers and paces sieges | Wealth, population and exported-wealth thresholds a player can edit; thieves and kidnappers come before armies; provoking a neighbour brings its attack sooner | S03: the first attack is a raid of 2-3, later ones grow with population, wealth and sieges broken | No difficulty knobs, nothing for provocation, no sneak phase |
| Warning and surprise | Some come as hidden ambushes, others announce themselves; arrival time and map edge vary | Scouts always see the army about a day ahead; it arrives with the migrants | Never a surprise, and the timing is always the same |
| Commanders | Real office-holders of the enemy, named in the warning; an army routs when its leader falls | Have: a general from history, shown on a card; killing them routs the army (`siege_leader`, `rout`) | Generals have no pets, mounts or background shown |
| Army makeup | Main force, archers, miners, builders, thieves, war beasts, veterans and better kit as the threat grows | Kit per civilization (goblins, humans, elves with wooden arms, heavily armoured dwarf raiders) | Size grows but quality and roles don't |
| Getting past defences | Diggers tunnel through rock, builders raise stairs and floors, climbers scale rough walls, monsters tear down doors, bars, bridges and walls | Doors get battered (`break_door`); a raised drawbridge can't be passed | No digging, building, climbing or breaking walls (B06) |
| Learning | Invaders avoid corridors and traps that killed their comrades | None, and the overseer doesn't learn either (S04's open step) | Missing on both sides |
| Siege engines | Bolt throwers, catapults and ballistae firing in arcs, shattering stones, a siege-crew work detail; attackers bring battering rams | None | Missing (Inbox: siege workshop) |
| Talking first | An army can stop out of range and ask for an artifact or tribute; paying buys peace | None | Missing (B02, B05) |
| Thieves and kidnappers | Thieves get bolder with skill, plan routes, steal from the depot and take animals; snatchers carry children off to be raised as enemies | Skulkers steal the best loose item and bolt once seen within 5 tiles (`skulker_think`) | No kidnappers or skill tiers |
| Captives | Prisoners on both sides, rescue missions, and the caged freed by their comrades | Cage traps catch goblins and nothing more happens; a jail exists for justice | Missing (S03 captives) |
| From below | Organised cavern peoples raid while a cavern edge stays open | Troglodytes, spiders and cave horrors come up; cavern breaches get doors | No organised underground raids |
| Undead and night creatures | Necromancer hordes, zombie waves that snowball, werebeast raids, vampires | Have: F14 necromancer with risen dead (`game/necromancy.rs`) | No werebeasts or vampires (Inbox) |
| After the fight | The whole fort gets a good thought from victory, a fallen fort gets occupied, sieges go into history | Milestone banner for the first broken siege, siege notes (`game/annals.rs`), a fallen fort saved as a ruin | No mood lift, no occupation |
| Results in play | (n/a) | 58% of deaths happen during sieges; later sieges of about 8 still take forts; the bridge is up in 46% of first sieges and 58-86% of later ones | The fort can't turn the tide once attackers are inside |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| SIE-01 | The overseer learns from a siege | After each siege, it adds traps where the enemy came in, raises a second squad, and builds walls or a moat where the defence broke. This is the defender's mirror of invaders who learn | Planned (S04 open step) | M | ★★★ | S04 | WL S04; Log2025 (invaders learn); Req301 |
| SIE-02 | Unpredictable arrival | Attacks land on a random day within the season (not with the migrants) and along a wider stretch of the map edge | Partial: always day one of a season | S | ★★★ | A01 | Log2025 |
| SIE-03 | Victory cheer and feast | Breaking a siege gives every dwarf a good thought, and the overseer may give the winning squad a feast or extra drink | Partial: milestone banner | S | ★★★ | A01 | Log2025; Story:Mil#19 |
| SIE-04 | Freeing the caged | Invaders break open cages and cage traps holding their comrades | Missing | S | ★★★ | A01 | Log2025 |
| SIE-05 | Grudges with reasons | Raiding a people's sites, ambushing their caravans or felling their sacred woods brings a counter-attack sooner. The heralds say why, and a people wronged long ago comes back stronger | Partial: a broken siege brings a bigger one | S | ★★★ | B16 | Log2017/2019; Log2025; Story:Sie#7; Story:Sie#14 |
| SIE-06 | Meet the enemy leader | The attacker's background, motives, looks and deeds appear on the warning card instead of being buried in the legends | Partial: named general and card | S | ★★★ | B16 | Core15 |
| SIE-07 | Sneaks before armies | A civilization first sends thieves and kidnappers. Full armies come only once the fort is worth it, and a young outpost gets some protection | Partial: small first raid, skulkers | M | ★★★ | N02 | Log2009/2010; Core26 |
| SIE-08 | Baby-snatchers | Hidden kidnappers bag a child and run for the map edge. Years later the child may come back raised among goblins, holding goblin values, as an attacker | Planned (Inbox: kidnappers) | M | ★★★ | N02 | Core89; Req275; Req517; Bloat44; Log2006-19; Story:Sie#10 |
| SIE-09 | Parley at the gate | An army halts out of range to demand an artifact, tribute, a noble or a criminal. An envoy walks out under truce. Paying buys years of peace and refusing brings an assault. Either side can raise a white flag mid-siege | Planned (B02, B05) | M | ★★★ | B16 | 2020; 2025; Log2013/2017; Log2025; Story:Sie#5; Story:Sie#6 |
| SIE-10 | Wall-breakers | Big attackers smash doors, bars, raised bridges, furniture and in time built walls and fortifications. What a construction is made of decides how much it can take | Partial: doors battered, bridge invulnerable | M | ★★★ | B06 | Log2025; Req125 |
| SIE-11 | Sappers | Invaders with picks (trolls, ogres) tunnel slowly through rock, but only after every other route has failed | Planned (B06) | M | ★★★ | B06 | Log2025; Story:Sie#3 |
| SIE-12 | Siege builders | Attackers lay floors and stairs across open air and moats, even under the gap of a raised bridge | Planned (B06) | M | ★★★ | B06 | Log2025; Improved sieges |
| SIE-13 | Climbers and jumpers | Near their target, attackers climb rough walls and leap one-tile gaps. Smoothed walls can't be climbed | Planned (B06) | M | ★★★ | B06 | Bloat343; Log2012-14; Rel0.40 |
| SIE-14 | Invaders learn from losses | Attackers remember where comrades died or traps went off and re-plan around those spots, digging or climbing if they must | Planned (B06) | M | ★★★ | B06 | Req301; Improved sieges; Log2025 |
| SIE-15 | Battering rams | Armies haul crewed rams to smash gates, walls and buildings | Missing | M | ★★★ | B06 | Log2025; Bloat117 |
| SIE-16 | Bolt thrower | A defensive engine that fires ordinary bolts. Its operator can swing it to face any direction if there's room to stand behind it | Planned (Inbox: siege workshop) | M | ★★★ | A05 | Log2025; Log2009 |
| SIE-17 | Catapult with shattering stones | It fires within an arc, with some vertical reach, and its stones burst on impact into sharp shards | Planned (Inbox: siege workshop) | M | ★★★ | A05 | Log2025; Req475 |
| SIE-18 | Burning them out | Invaders set forests, fields and wooden works alight to smoke defenders out or ruin the surface | Missing (fire exists) | S | ★★★ |  | Story:Sie#4 |
| SIE-19 | Traps jammed with bodies | A trap corridor stops working once carcasses pile up, and someone has to clear it under fire | Missing | S | ★★★ | A01 | Story:Construction#1; Bloat295 |
| SIE-20 | Drowning the invaders | The overseer floods a gate corridor or trap hall through floodgates when attackers fill it | Partial: lever floodgates exist, unused for this | S | ★★★ | A01 | Story:Construction#5 |
| SIE-21 | Poisoned water | Saboteurs or evil creatures foul the fort's well or cistern | Missing | M | ★★★ | N02 | Bloat272 |
| SIE-22 | Long sieges and siege camps | Besiegers dig in with a camp and a command tent, turn out when the alarm sounds, send search parties after anyone who slips out, and bombard for days. A starving fort eats whatever it can, the dead included | Missing | M | ★★★ | A05 | PG4; Log2006-19 (camps); Story:Sie#2 |
| SIE-23 | Flying raiders | Riders on giant bats or winged beasts ignore the drawbridge and have to be met with archers, roofs and nets. Fliers retreat cleanly | Missing | M | ★★★ |  | Story:Sie#9; Log2025; Req324 |
| SIE-24 | Carried off alive | Invaders drag dwarves away as captives. A rescue can free them, or they return years later changed | Planned (S03 captives; B03) | L | ★★★ | B16 | Story:Sie#13; Log2014 |
| SIE-25 | Refugees ahead of the army | Starving migrants fleeing a war elsewhere arrive, followed by the army that drove them out | Missing | M | ★★★ |  | Story:Sie#15; Log2006-19 (refugees) |
| SIE-26 | Bandits, cults and rackets | Small groups outside any civilization harass, raid or extort a young fort and take a cut of its trade. Cults grow inside a big one | Missing | M | ★★★ | N02 | Core71; 2020; Story:Economy#7 |
| SIE-27 | Assassins and spies | Agents of a rival or villain, sometimes hidden among migrants, target nobles and heroes with poison, ambushes or staged accidents | Planned (B04); Partial: goblin-spy mystery | M | ★★★ | N02 | 2020; Story:Justice#10-11 |
| SIE-28 | The exile returns | A banished dwarf comes back with bandits or goblins and uses what they know of the fort | Missing | M | ★★★ | N02 | Story:Justice#2 |
| SIE-29 | Raids from below | Organised cavern peoples (animal folk with crude spears, bat riders with poison darts, antmen with a queen) keep raiding while a cavern edge stays open | Partial: troglodytes, cave horrors | M | ★★★ |  | Req232; 2025; Log2008/2015/2019; Story:Digging#1 |
| SIE-30 | Monsters that come for you | Megabeasts hunt the fort out, flight matters, and dragons snatch dwarves and drop them from a height | Partial: megabeasts hunt, dragons breathe fire | M | ★★★ |  | Core40; Req324; 2025 |
| SIE-31 | Werebeast raid | Under the full moon a werebeast rushes the fort, savages a few dwarves and flees. Those bitten turn at the next full moon | Planned (Inbox: werebeasts) | M | ★★★ |  | Log2011-19; Core96 |
| SIE-32 | Evil armies with character | Generals ride giant beasts and keep favourite monsters, and now and then a rival army turns up and fights them at your gate | Partial: generals from history | L | ★★★ | B16 | Bloat68 |
| SIE-33 | Armies you can watch coming | Attackers march from a real site across the world map; you can judge their size and kind there, and kills thin that site's next army | Planned (B01) | L | ★★★ | B16 | Core26; 2011 III.A.2; 2020 Army; Story:Interface#3 |
| SIE-34 | Feints and fall-backs | Attackers pull back to lure defenders out of the walls into an ambush. A Warlord overseer can do the same into a trap hall | Planned (B06, partly) | M | ★★★ | B06 | Story:Mil#15 |
| SIE-35 | Narrow passages | Narrow doorways stop large creatures (who may smash through), and cramped tunnels hamper archers | Missing | M | ★★★ |  | Story:Construction#3 |
| SIE-36 | Beasts of war and leaders' pets | Armies bring tamed cave creatures, bears and trolls. Leaders keep named pets chosen to suit their tastes and their homeland | Missing | S | ★★ | B16 | Core11; Log2025; Log2006-19 |
| SIE-37 | Armies with roles | A main force plus archers, miners, builders and a thief group. Support groups hold while needed and leave when their part is done or routed | Partial: kit per civilization | M | ★★ |  | Log2025 |
| SIE-38 | Veterans in the ranks | As the threat grows, later or richer targets draw better-trained, better-armed soldiers instead of just more of them | Partial: size grows, not quality | S | ★★ |  | Improved sieges; Log2025 |
| SIE-39 | Cut-off leaders adapt | A commander whose route is blocked tries something else (loot, burn, dig, camp) instead of standing still | Partial: shut-out besiegers loot outside | S | ★★ | B06 | Log2025 |
| SIE-40 | Reinforced walls | A wall built from two blocks plus a metal bar is far tougher. A construction's material sets how much damage it takes | Missing | S | ★★ | B06 | Log2025 |
| SIE-41 | Siege dials | Settings for invader digging, building and wrecking, attack frequency, wave size, monster caps and wealth triggers. They make autoplay batches tunable | Partial: thresholds hard-coded | S | ★★ |  | Log2022; Log2025; Log2007-15 (size caps, thresholds) |
| SIE-42 | Ambush vs open siege | Some forces hide until they strike while others announce themselves. Tracks, a silent wood or a scout's luck can give an ambush away. The siege history records which kind it was | Partial: every army is scouted a day early | S | ★★ | A01 | Bloat131; 2009 Req; Log2006-15 |
| SIE-43 | Drawn from real populations | Besiegers and their mounts are counted out of an actual enemy site, so heavy losses weaken its next attack | Partial: generals are real, troop counts aren't | M | ★★ |  | Log2010s; Log2025 |
| SIE-44 | Thieves who plan | Thieves and their escorts plot a route around guards and dwarves, keep their distance, and can be ambushed themselves | Partial: skulkers flee when spotted | S | ★★ | N02 | Bloat103; Log2010s |
| SIE-45 | Thief ranks | Thieves look and act differently as their skill grows, and the best are bold and inventive | Missing | M | ★★ |  | Bloat86; Bloat114; Req158 |
| SIE-46 | Wider theft and rustling | Thieves take goods waiting at the depot and lead animals out of the pastures at night (to keep as pets or carve into trinkets) | Partial: skulkers take the best loose item | S | ★★ | N02 | Bloat113; Story:Agri#3 |
| SIE-47 | Distraction thieves | A thief knocks, makes noise or sets a fire elsewhere to draw the guards before grabbing the prize | Partial | S | ★★ | N02 | Story:Sie#11 |
| SIE-48 | Kobold raids | Small trickster thieves who set traps, carry poisonous vermin, hoard shiny things and taunt their marks | Missing | M | ★★ | N02 | Core90; 2025; Log2008; Story:Creatures#13 |
| SIE-49 | Caravan raiders | Bandits waylay caravans and the guards fight back. A bribed merchant leaks what the next caravan will carry | Missing | M | ★★ |  | Caravan Arc III.B.1; PG24; PG33 |
| SIE-50 | Provoking the deep and the river | Heavy work at a river bank or chasm edge (dumping, bridging) draws attacks from whatever lives there | Missing | S | ★★ |  | Req8 |
| SIE-51 | Mounted invaders | Peoples who keep horses ride them to war | Missing | M | ★★ |  | Bloat289 |
| SIE-52 | Trophies and terror | Heads on poles, skull banners and impaled foes frighten the faint-hearted on both sides and enrage the victims' kin | Missing | S | ★★ |  | Story:Sie#12; Story:Crafts#3 |
| SIE-53 | Abusing envoys | Diplomats and merchants can be insulted, jailed or killed, with lasting consequences (fits Reckless overseers) | Missing | S | ★★ |  | Bloat31 |
| SIE-54 | Siege crews | A default siege-operator work detail. Operators stay at their posts, practice-fire, and leave when not ordered there. One list shows every engine and its state | Missing | S | ★★ | A05 | Log2025 |
| SIE-55 | Attackers' siege engines | Besiegers build and bring engines (rams with carved animal heads, catapults) assembled from parts, and bombard before the assault | Missing | L | ★★ | A05 | Bloat117/156; Story:Sie#2 |
| SIE-56 | Coalition armies | Several peoples march together under one grievance (elves with animal folk, goblins with trolls) | Missing | L | ★★ |  | Story:Sie#8 |
| SIE-57 | Occupation and uprising | A fort can fall without dying: the victors install an overlord and collaborators, loot the place and garrison it. Losses or rumours of resistance can spark an uprising | Missing | L | ★★ |  | Log2013-19; Rel0.42; Req61; Story:Sie#16 |
| SIE-58 | Magical sieges | Dwarves taken by demons come back as sorcerers among the invaders, and magical beings besiege the fort | Planned (B14) | XL | ★★★ |  | 2020; 2025 |
| SIE-59 | Theft totals | A running tally of what thieves have carried off | Missing | S | ★ |  | Bloat107 |

*Already Have and left out of the table: scouts' warning with a named leader (Story:Sie#1, S03), commanders from history (Log2025), contact that follows world diplomacy (Log2006-19), necromancer sieges and the dead rising (Core40, Log2025, F14). Dropped as engine trivia: none.*

