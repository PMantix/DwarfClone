# Economy: trade, crafts, farming and engineering

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## Economy, trade & industry (ECO)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Caravan arrival | Seasonal caravans of each friendly civ arrive by wagon or pack animal; wagons need a wide path to the depot; later plans have them travel the world map from real sites | One caravan per season (elves spring, humans summer, own civ autumn) if at peace and within reach; two merchants walk in from the map edge and stay six days (`game/trade.rs` `caravan_season`) | No travel on the world map, no wagons or pack animals, no caravan size |
| Caravan goods | Goods drawn from the home site's actual stocks and the civ's resources; caravans skip what you already have plenty of; sized clothing | Fixed list per race (`caravan_goods`) | Goods never respond to the fort's needs or the world |
| Bartering | Item values, merchant mood, counter-offers, an appraiser, offerings to the civ | Barter screen with a 10% markup; broker cuts it by fit (`trade_margin`, `offer_ok`) | No mood, counter-offers or explanation of a refusal |
| Trade agreements | Liaison sets next year's requests and wants; export agreements pay a premium on chosen goods; export bans | None | No agreements, no requests, no bans |
| Prices | Each civ prices goods by its own scarcity and wants; food prices too | Fixed value per kind and material times quality (`items.rs`) | No local or shifting prices |
| Trade as diplomacy | Elves resent wood and felled forests; unprofitable trade tapers off; harming merchants or envoys has consequences; noble rank grows caravans | Killing every merchant just spills their goods (`merchant_died`); word-of-mouth via merchants (`game/word.rs`) | Conduct never changes relations or caravan frequency |
| Who trades | The player, helped by a broker | The overseer marks all goods for trade and buys food first, then metal, then cloth (`game/overseer.rs` `ov_trade`) | The overseer ignores its own troubles list and its traits when buying |
| Money | Coins minted and once paid as wages, rents and bonuses (later dropped) | No currency | Missing (and DF dropped most of it) |
| Work orders | Conditional orders (stock thresholds, after another order), repeat schedules, manager approval, auto-filled conditions | "Keep N in stock" orders at the least-busy workshop, needing a manager, frequency by bookkeeper (`game/orders.rs`) | No conditions, chains, counts or input queuing; waiting orders spam |
| Job assignment | Labors, work details, job priorities, dig priorities, workshop masters and skill minimums | 20 labors, work details, workshop profiles of who may work (`game/labor.rs`, `game/rooms.rs`) | No priorities, no masters or minimum skill |
| Workshops | About 30 kinds plus data-defined custom workshops and reactions | 18 kinds, 49 recipes in code (`buildings.rs`) | No multipurpose or data-defined workshops |
| Stockpiles | Fine filters incl. quality and wear, give/take links, feeding workshops, bins, barrels, an "everything" pile | Kind and material refusals, forbid and dump, bins and barrels (`game/rooms.rs` PileToggle) | No links, no quality filter, no consolidation |
| Hauling | Bins carried whole, wheelbarrows, minecarts on tracks with routes, multiple haulers | One item per trip (`game.rs` `find_haul`) | All logistics upgrades open (B11) |
| Wealth effects | Created and exported wealth pace nobles, migrants, megabeasts and sieges | Wealth and population raise titles and siege strength; megabeasts after year one | Exported wealth not tracked separately |
| Mining logistics | Vein-following designations, dig priorities, stone dumping | Dig designations only (`world.rs` Desig) | No vein auto-mining or priorities |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| ECO-01 | Starving dwarves raid the caravan | A dwarf desperate with hunger or thirst grabs food or drink off the depot; if the fort can't cover the price, the civ's opinion drops. | Missing | S | ★★★ | B15 | Bloat#230; roadmap Bloat230 |
| ECO-02 | Harming merchants has fallout | Killing, robbing or caging merchants (by a mad dwarf, a Reckless overseer or a trap) sours or ends relations and can bring war. | Partial: a slain caravan spills its goods, no diplomatic effect (`trade.rs`) | S | ★★★ | B15 | Bloat#31; roadmap Bloat31 (abusing envoys); Story:Animal Justice |
| ECO-03 | Caravans bring what you lack | The caravan's list leans toward the fort's open troubles (no food when stores are full, more of what is short), and the overseer buys against its troubles list. | Partial: fixed race lists, overseer buys food then metal | S | ★★ | B15 | Log2023; Req555; Core38 |
| ECO-04 | Conditional work orders | Orders that fire when a stock falls below N or another order finishes, repeat daily/monthly/yearly or N times, with an "auto" button filling sensible conditions; the overseer writes them as its plans. | Partial: keep-N-in-stock orders (W14) | M | ★★★ | O08 | Log2021; Log2016/Rel0.43; Req2; roadmap standing orders 2012-16 |
| ECO-05 | Neighbours judge your conduct | Elves count felled trees, wooden goods and displayed hides; other civs note executions and trophies; caravans shrink, stop or turn hostile. | Missing | M | ★★★ | B15 | Story:Animal Justice; Req132; Req463; Req516; Bloat#72 (roadmap); Log2025 |
| ECO-06 | Haggling explained | Counter-offers come from the broker's skill against the merchants' mood and profit, and the screen and overseer notes say why a deal failed. | Partial: broker cuts markup | S | ★★ | B15 | Log2023; Req260; Req279 |
| ECO-07 | Trade that dies off | If merchants make no profit for a few visits, caravans come less often and smaller. | Missing | S | ★★ | B15 | Log2026 |
| ECO-08 | Wary caravans | Merchants check the depot approach for ambush and refuse to enter a fort with a bad name or bodies on the road. | Missing | S | ★★ | B15 | Story:Animal Justice |
| ECO-09 | Nobility drives trade | Each noble rank enlarges the home caravan; a barony opens trade or diplomacy with far civs. | Partial: baron/count/duke by wealth, no effect on caravans | S | ★★ | B15 | Log2025 |
| ECO-10 | Export bans | A noble or the civ forbids selling certain goods; selling them anyway angers the noble (a Greedy overseer may risk it). | Missing | S | ★★ | B15 | Log2025; Rel0.31 |
| ECO-11 | Orders queue their own inputs | An order missing thread queues spinning first instead of failing and spamming; impossible jobs show what is missing. | Partial: Inbox note on order spam | M | ★★ | O08 | Bloat#253; Req2; Rel0.31 (missing reagents shown) |
| ECO-12 | Mercenaries and monster hunters for hire | Companies with reputations take contracts (hunt the cave horror, guard the gate) for pay or goods; they can fail or cheat. | Planned: F07 (hunters) | M | ★★★ | P04 | Story:Monster Quest; Log2016 (mercenary companies); Log2024 (companies in history) |
| ECO-13 | Workshop masters and apprentices | A workshop gets a permanent master, a skill minimum and apprentices who learn faster there. | Partial: workshop profiles; Inbox min skill | S | ★★ | O08 | Bloat#32; Req560 |
| ECO-14 | Vein mining | Mark a vein or gem cluster and the designation spreads as miners uncover it, with ore-only or gem-only filters. | Missing | S | ★★ | O08 | Bloat#218; Req393; 2016; Rel0.40 |
| ECO-15 | Records of production | The fort keeps a ledger of who made what, read in the chronicle or after the fall. | Partial: chronicle, legends | S | ★★ | O08 | Core70; Rel0.31 |
| ECO-16 | Thieves at the depot | Skulkers go for goods marked for trade and for small animals. | Partial: skulkers steal the best loose item | S | ★★ |  | Bloat#113 |
| ECO-17 | Exported wealth counts | Exported wealth is tracked apart from created wealth and pushes titles, migrants and attention. | Partial: titles by total wealth | S | ★★ |  | Rel0.31; Log2016 (tunable thresholds) |
| ECO-18 | Selling captives | Caged beasts or prisoners can be sold; their later fate (a pet, an arena) can come back as news. | Missing (needs S03 captives) | S | ★★ |  | Req110 |
| ECO-19 | Tribute through caravans | Tribute owed or demanded travels in real caravans that can be raided or withheld. | Planned: B02 | M | ★★★ | B24 | Core81; 2016-2020 |
| ECO-20 | Job priorities and postings | Jobs advertise and the best free dwarf takes them; urgent jobs pull workers off minor ones; dig designations carry a 1-7 priority. | Missing | M | ★★ | O08 | Bloat#54; Req86; Req428; 2016; Log2016/Rel0.40 |
| ECO-21 | Trade agreements | The liaison agrees what next year's caravan brings and what the civ wants; export agreements pay a premium on one or two goods. | Planned: B02 | M | ★★ | B24 | Req555; Log2010/Rel0.31; Log2025 |
| ECO-22 | Messengers open trade | A messenger sent from the world screen can open trade early with a neighbour or improve terms. | Planned: B02 | S | ★★ | B24 | Log2026 |
| ECO-23 | Local prices | Each civ prices goods by its own scarcity and tastes; food prices move too. | Missing | M | ★★ | B24 | Bloat#155; Core38; Story:catalogue |
| ECO-24 | Selling artifacts and collectors | Some figures collect masterworks, coins or curios; finding a rich buyer for an artifact is a venture in itself. | Planned: B05 | M | ★★ |  | Bloat#173; Bloat#263; Bloat#163; Core79 |
| ECO-25 | Contracts with part payment | Half up front, the rest on delivery; the hired side may vanish with the first half. | Missing | S | ★★ | P04 | Story:Curse of the Cyclops |
| ECO-26 | Patrons fund works | A rich noble pays for a hall or monument in their own name. | Missing | S | ★★ | P04 | Story:catalogue |
| ECO-27 | Protection rackets | Bandits or a rival demand a cut of each caravan's trade; refusing invites raids. | Missing | M | ★★ | P04 | Story:Curse of the Cyclops |
| ECO-28 | Caravan raiders | Bandits waylay caravans on the road; a bribed merchant leaks the next cargo. | Missing (after B01) | M | ★★ | P05 | Arc Caravan III.B.1; PG24; PG33 |
| ECO-29 | Minecarts and hauling routes | Carved or built tracks, rollers and stops; routes with departure conditions; carts that speed downhill and fly off corners. | Planned: B11 | L | ★★ | B11 | Bloat#59; Log2021; Log2012/Rel0.34; Hauling 2012-20 |
| ECO-30 | Fairs | Seasonal markets where several caravans meet at once. | Missing | M | ★★ | P05 | 2012 R4; Trader role |
| ECO-31 | Your own caravans | The fort sends wagons out to sell to neighbours. | Missing | M | ★★ | P05 | Req425; Arc Merchant; Trader role |
| ECO-32 | Caravans on the world map | Caravans travel visibly between sites by road and carry what you sold. | Planned: B01 | L | ★★ | P05 | Core3; Arc Caravan; 2011 |
| ECO-33 | Hill and deep dwarf partners | Nearby dwarf settlements, above and in the deep roads, trade food and send migrants. | Planned: B03 | L | ★★ | P05 | 2016-2020 scenarios; 2018; Log2012 (deep dwarf sites) |
| ECO-34 | Material-specific orders | An order for a named stone, wood or plant uses exactly that, which matters for noble demands. | Partial: stockpile filters | S | ★ | O08 | Log2023; Req27; Req287; 2012 |
| ECO-35 | Mandates feed orders | A noble's production mandate can be sent straight into the work orders. | Missing | S | ★ | O08 | Bloat#250; Bloat53 (roadmap) |
| ECO-36 | Stockpile links | Piles give to and take from chosen piles or workshops only. | Planned: Inbox stockpile links | S | ★ | B11 | Req1; Log2012/Rel0.31; Log2021 |
| ECO-37 | Quality filters and tidying | Piles accept by quality or wear, a meltables pile, and an order to pack a pile's items together. | Partial: kind and material filters | S | ★ | B11 | Bloat#308; Bloat#313; Req528 |
| ECO-38 | Everything pile and repainting | One click makes an all-goods pile; piles can be resized or repainted. | Partial: W14 | S | ★ |  | Log2021 |
| ECO-39 | Finer labours | Separate labours for lever pulling, water hauling, trade hauling and deconstruction; burrows limit where a workshop fetches. | Partial: 20 labors | S | ★ |  | Rel0.31 |
| ECO-40 | Automated processing | Workshops tan hides, clean fish and render fat unbidden; a kitchen keeps a few barrels of its own. | Partial: work orders | S | ★ |  | Rel0.31; Bloat#60 |
| ECO-41 | Disposal orders | Standing rules for worn clothes and captured gear after a siege. | Missing | S | ★ |  | Req436 |
| ECO-42 | Workshop task tooltips | Adding a task shows its product and inputs. | Missing | S | ★ | O08 | Log2022; Rel0.31 |
| ECO-43 | Embark goods and warnings | Starting kit includes trade goods; saved embark profiles; warnings about missing clay, flux or ore. | Partial: embark kit, aquifer shown | S | ★ |  | Log2006; Rel40d; Rel0.31 |
| ECO-44 | Prices meter growth | Caravan balance and prices help keep population from running away. | Partial | S | ★ |  | Log2023 |
| ECO-45 | Anything is for sale | Any owned item can be bought at a high enough price. | Partial: depot barter | S | ★ |  | Bloat#267 |
| ECO-46 | Multi-item hauling | Haulers carry several items or a whole bin and top up containers of the same thing. | Planned: B11 | M | ★ | B11 | Core56; Log2012 |
| ECO-47 | Wheelbarrows | Stone piles request wheelbarrows; one trip moves heavy loads. | Planned: B11 | M | ★ | B11 | Bloat#10; Log2010/Rel0.31 |
| ECO-48 | Burrow logistics | Burrows hold stock targets, transfer piles move goods between them, hauling a trade of its own. | Partial: burrows, alert burrow | M | ★ | B11 | Core36; Req22; Req24 |
| ECO-49 | Multipurpose workshop | One general shop for simple jobs; added tools turn it into a specialist. | Missing | M | ★ |  | Bloat#19 |
| ECO-50 | Data-defined workshops | Workshops and reactions in data files with labour and skill needs. | Missing | M | ★ |  | Rel40d; Story:catalogue (moddable definitions) |
| ECO-51 | Rooms for rent | Bedrooms and dining rooms let to residents and guests, rents by supply and demand, nobles bidding, an inn's reputation. | Planned: B08 | M | ★ | P04 | Bloat#7; Bloat#8; Bloat#9; Bloat#83; 2012 R8; 2016 taverns; Story:catalogue (inns) |
| ECO-52 | Loans and creditors | Debts with interest; an unpaid creditor turns enemy. | Missing | M | ★ | P04 | Story:The Mighty Savager |
| ECO-53 | Coins and wages | Minted coins, wages, bonuses for fine or dangerous work, a treasurer merging coin piles. | Missing (DF later dropped it) | L | ★ | P04 | Bloat#79; Bloat#96; Bloat#251; Bloat#158; Req100; Req102; Req394 |
| ECO-54 | World economy | Sites produce and consume each year; caravans carry real stocks; your exports show up in far markets. | Missing | L | ★ | P05 | Core38; Log2012; Trader role |
| ECO-55 | Merchant companies | Trading houses open branches and sometimes a warehouse inside the fort. | Missing | L | ★ |  | Log2016; Log2024 |

*Dropped (pure engine): Bloat#215 coin batch fix, Bloat#120 job stat storage.*

## Crafts, materials & items (CRF)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Item kinds | Hundreds: furniture, tools, instruments, toys, trinkets, books, siege parts | 50 ItemKinds (`items.rs`) | No instruments, toys, trinkets, display furniture |
| Materials | 200+ rock types, many metals and gems, each with density, hardness, edge and melting point | ~25 Mats: a few stones, 4 ores, coal, 5 metals, one gem, wood, bone, leather (`material.rs`) | Thin list; no physical properties |
| Workshops | ~30 incl. kiln, glass furnace, jeweler, dyer, ashery, bowyer, siege workshop | 18 kinds incl. magma smelter and forge (`buildings.rs`) | No kiln, glass, jeweler, dyer |
| Quality and value | Six quality steps, plus decorations each with its own quality | Quality 0-5, artifact 6, value multiplier (`items.rs`) | No decorations on items |
| Decoration and images | Studs, encrustings, bands and images; image detail grows with skill; images of historical events | Engravings on smoothed rock show legend scenes (`game/detail.rs`) | Items carry no images; built walls can't be engraved |
| Artifacts | Mood artifacts named, claimed by families and civs, stolen, displayed, inherited, sought | Mood artifacts named, quality 6 (`game/psyche.rs`); ruin vault artifact (`game/ruins.rs`) | No claims, heirlooms, display or theft of artifacts |
| Trophies | Bone, skull, hide and horn crafts with the source creature named; skull totems | Dragon skull treasure; bone crafts (`game/combat.rs`, recipes) | No trophies of slain foes in halls |
| Butchery yield | Meat, organs, fat, bone, skull, horn, teeth, ivory, shell, hide sized to the beast | Meat, bone, hide, tallow | Few by-products; hide not by size |
| Recycling | Melt metal items (masterworks upset their maker), break furniture down | None (Inbox melting) | Captured gear piles up unused |
| Clothing | Layers (under, over, cover), many garments by culture, insulation, dyes, wear | Shirt, trousers, shoes that wear out in a year (`game/clothing.rs`) | No layers, gloves, cloaks, dyes, fashions |
| Wear and decay | Clothes, weapons and armour wear; items rot and age; contaminants | Clothes wear; corpses rot (`game/miasma.rs`) | Gear never wears; no rust |
| Processing chains | Lye, potash, pearlash, oil, soap, clay, glass, plaster, paper | Ash, tallow, soap, charcoal, flour, thread, cloth | No ceramics, glass, oil, writing materials |
| Display | Pedestals and display cases make museums and halls | None | Missing |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| CRF-01 | Trophies of war | Skulls, hides, horns and banners of slain foes become crafts or hall displays that cheer dwarves and enrage the victims' kin; enemies hang trophies made from your dead. | Partial: dragon skull treasure | S | ★★★ | O03 | Bloat#161; Bloat#165; PG19; PG181; Story:Animal Justice; Story:Heroes of the Coast |
| CRF-02 | Named weapons | A weapon that kills enough gets a name; its wielder grows attached and others may covet it. | Missing | S | ★★★ | O03 | Story:catalogue; PG133; Core99 |
| CRF-03 | Blueprints | An inspired engineer draws plans for a grand work, which the overseer may adopt as an Architect ambition. | Missing | M | ★★★ | O12 | Story:Half Dwarf; Story:catalogue |
| CRF-04 | Heirlooms and tokens | Items tied to a person pass to kin; one found away from its owner tells of their fate. | Planned: B05 | M | ★★★ | B05 | Story:Gorge of the Righteous; Story:Animal Justice; 2016; 2020 artifacts; Rel0.44 |
| CRF-05 | Artifact claims | Families, civs and faiths claim artifacts, which drives theft, demands, envoys and war. | Planned: B05 | M | ★★★ | B05 | Rel0.44; 2016; 2020; Story:Dwarven Assault |
| CRF-06 | Inscriptions and maker's marks | Dwarves carve names, prayers or taunts on gear; blades carry a maker's mark and spark quarrels over ownership. | Missing | S | ★★ | O03 | Story:catalogue; PG133; Core99; Bloat#260 |
| CRF-07 | Display furniture | Pedestals and cases show artifacts and fine goods; a museum room can sit inside a tavern, temple or library. | Missing | S | ★★ | B05 | Log2020; Rel0.47; 2016 artifacts |
| CRF-08 | Name creator | Artifacts, weapons and places get generated names the overseer can use for halls and gear. | Missing (O03 named halls) | S | ★★ | O03 | Log2022 |
| CRF-09 | Artifact sets | Some artifacts come as matched sets: a quiver of bolts, a suite of jewellery. | Missing | S | ★★ | B05 | Bloat#49 |
| CRF-10 | Melting and recycling | Captured gear marked for the (magma) forge returns bars; furniture broken down; flux recovered; melting a masterwork upsets its maker. | Planned: Inbox melting | S | ★★ | D08 | Log2022; Bloat#265; Bloat#363; Rel40d |
| CRF-11 | Rust and tarnish | Neglected metal rusts or greens, lowering value and edge until cleaned. | Missing | S | ★★ | X02 | Bloat#145 |
| CRF-12 | Gems in engravings, engraved built walls | Engravings set with gems or metal; built walls can be smoothed and engraved like rock. | Missing | S | ★★ | O03 | Bloat#245; Log2023 |
| CRF-13 | Jeweler and decorations | A jeweler sets gems into furniture and gear, each decoration with its own quality and value. | Planned: Inbox jeweler | S | ★★ | O03 | Bloat#2; inventory gap |
| CRF-14 | Toys and games | Wooden toys and board games; children play with them and copy sports heroes. | Missing | S | ★★ | D08 | Bloat#41; Story:Dwarven Servitude; Story:catalogue |
| CRF-15 | Musical instruments | Instruments of several parts and materials, by culture, played in the tavern. | Missing | S | ★★ | D08 | Log2020; Rel0.42 |
| CRF-16 | Fossils | Fossils and petrified wood turn up in digging and are prized by collectors. | Missing | S | ★★ |  | Bloat#324 |
| CRF-17 | Found treasures | Jade in some rivers, pearls, amber, horn and ivory as rare materials. | Partial: bone crafts, dragon hoards | S | ★★ |  | Bloat#322; roadmap Bloat322 |
| CRF-18 | Restraints | Rope, chains and cages to bind prisoners and beasts in jail or pasture. | Planned: Inbox jail chains | S | ★★ |  | Story:catalogue; Story:Animal Justice |
| CRF-19 | Wall hangings and paintings | Rugs, tapestries, paints and pigments that raise room value. | Missing | S | ★★ |  | Bloat#165 |
| CRF-20 | Living and trick statues | Statues with hidden levers or moving arms, and rare ones that wake. | Missing | M | ★★★ |  | Bloat#224 |
| CRF-21 | Statues and figurines of things | Figurines and statues depict creatures, objects, buildings and engines, not just scenes. | Partial: legend engravings, monument | S | ★★ | O03 | Bloat#152; Bloat#116; Log2021 |
| CRF-22 | Art critics | Remarks on an artist's works track a career of rising or falling taste. | Missing | S | ★★ |  | Bloat#22 |
| CRF-23 | Etching and runes | Etched designs on items, a path toward magic runes. | Partial: engraved walls | S | ★★ |  | Bloat#97 |
| CRF-24 | Layered pictures | An item's image has parts that take further decoration (ruby eyes on a carved tortoise); detail scales with skill; motifs like eyes or hands. | Partial: quality, engravings | M | ★★ |  | Bloat#2; Bloat#221; Bloat#302; Bloat#1; Rel40d |
| CRF-25 | Whole-carcass butchery | Carcasses yield organs, fat, skull, horn, teeth, ivory, shell and chitin, each usable; skull totems from heads. | Partial: meat, bone, hide, tallow | M | ★★ | D08 | Rel0.31; Bloat#246; Bloat#104; Log2023 (shell crafts) |
| CRF-26 | Artifacts from history | World history already holds mood artifacts, heroes' named gear and relics that can turn up. | Partial: ruin vault artifact | M | ★★ |  | Rel0.42; Rel0.44 |
| CRF-27 | Clothing variety and fashions | Each people dresses its own way in many garments and colours; trade blurs it; dwarves remark on strangers' dress. | Partial: shirt/trousers/shoes; Inbox cloaks | M | ★★ | X02 | Bloat#243; Bloat#242 |
| CRF-28 | Automatons | Clockwork servants, best made in a fey mood. | Missing | M | ★★ | O12 | Bloat#42; 2020 |
| CRF-29 | Masterwork history | Making or losing a masterwork enters the legends. | Partial: first masterwork milestone | S | ★ |  | Rel0.31 |
| CRF-30 | Gear wear and grime | Weapons and armour wear in fights; blood and mud on gear need cleaning and unsettle onlookers. | Partial: clothes wear (W12) | S | ★ | X02 | Req149; Req431; Log2023; Story:catalogue |
| CRF-31 | Leather done right | Hide size follows the creature and yields several leathers; tawing and waterproofing. | Partial: tanner's shop | S | ★ | D08 | Bloat#257; Bloat#258; Bloat#259; Req91; Log2023 |
| CRF-32 | Oil, potash, lye | Nuts pressed to oil, ash to potash and lye, broadening soap and fertiliser chains. | Partial: soap from ash and tallow | S | ★ | D08 | Rel0.31 |
| CRF-33 | Keys and locks | Real keys for locked doors; lost keys in old ruins. | Partial: lever-locked doors | S | ★ |  | Bloat#188 |
| CRF-34 | Comfortable beds | Beds of more materials with blankets and pillows. | Missing | S | ★ |  | Bloat#40 |
| CRF-35 | Luxury trinkets | Mirrors, clocks, masks, dice and cards. | Missing | S | ★ | O12 | Bloat#34; Bloat#39 |
| CRF-36 | Decor shows the owner | Gloomy nobles pick sinister furniture, cheerful ones bright. | Missing | S | ★ |  | Story:catalogue |
| CRF-37 | Shoddy primitive loot | Goblin and kobold gear is old, rotten or crude. | Missing | S | ★ |  | Story:catalogue |
| CRF-38 | Gem cuts by culture | Each civ has its own cuts; plain stones are cut as cabochons. | Missing | S | ★ |  | Log2008 |
| CRF-39 | Left and right | Gloves and other pairs come as left and right; odd mittens and fingerless gloves. | Missing | S | ★ | X02 | Bloat#84; Bloat#296 |
| CRF-40 | Deliberately rough work | Order an item below one's best. | Missing | S | ★ |  | Bloat#298 |
| CRF-41 | Exact bar counts, metal blocks | Bars counted one by one per job; metal blocks where stone blocks are asked. | Partial: bars per job | S | ★ |  | Bloat#6; Bloat#310 |
| CRF-42 | Item descriptions | Items describe their decorations and history. | Partial: quality and artifact names | S | ★ |  | Bloat#1 |
| CRF-43 | Ageing and auto-forbid | Items age smoothly over time; gear of the dead and spent bolts are forbidden automatically. | Partial: rot, manual forbid | S | ★ |  | Bloat#220; Rel0.34 |
| CRF-44 | More metals and gems | Silver, tin, zinc and many gems, opening alloys and decoration choices. | Partial: 5 metals, 1 gem | M | ★ | D08 | Story:features; inventory gap |
| CRF-45 | Kiln, ceramics, lime | Clay to earthenware, stoneware or porcelain; glazed jugs and big pots as barrels; quicklime and plaster. | Planned: Inbox kiln | M | ★ | D08 | Bloat#77; Bloat#229; Rel0.31; Story:features |
| CRF-46 | Glass furnace | Sand gathered in bags makes glass, glass goods and windows. | Planned: Inbox glass | M | ★ | D08 | Rel40d; Bloat#34 |
| CRF-47 | Dyes and painted designs | Dye plants and mixed colours tint cloth and leather; painted armour and banners; piles filter by colour; sewn images. | Planned: Inbox dyes | M | ★ | X02 | Bloat#264; Req400; Log2025; Log2008; Story:Forest Befouled |
| CRF-48 | Clothing layers and warmth | Under, over and cover layers that insulate and are chosen by climate. | Partial: one layer | M | ★ | X02 | Rel0.31 |
| CRF-49 | Heat melts metal and glass | Fire and magma melt metal and glass items, not just burn wood; exotic plants melt. | Partial: fire burns items | M | ★ |  | Story:Mother of Death; Story:catalogue; Bloat#256 |
| CRF-50 | Coal gas and tar | Coking gives gas that sickens with exposure and tar for soap. | Missing | M | ★ |  | Bloat#63 |
| CRF-51 | Cut-to-length goods | Thread, rope, chain and cloth in lengths that jobs use up. | Missing | M | ★ |  | Bloat#297 |
| CRF-52 | Writing materials | Papyrus, pulp paper and parchment become quires, scrolls and bound books. | Planned: B08 (books) | M | ★ |  | Rel0.42; Story:features |
| CRF-53 | Real material properties | Every material has density, hardness and edge; mineral hardness changes dig speed. | Partial: material table | L | ★ |  | Rel0.31; Bloat#282 |

*Dropped: none.*

## Agriculture, food & animals (AGR)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Crops | Dozens of crops by biome and season, above and below ground, with seasonal growths on trees and shrubs | 5 crops with seasons and surface/underground places (`material.rs` Crop, `game.rs` grow_farms) | Few crops; no tree growths or orchards |
| Soil | Planned moisture, irrigation, nutrients, fertiliser, weeds; shallow soil far above caverns farms poorly | Farm plots that remember a crop (`game/farming.rs`) | No soil quality at all (B12) |
| Flood and water | Floodgates water fields; rivers flood land | Floodgates exist but don't touch farming | Missing |
| Gathering and trees | Multi-tile trees fall away from the cutter; fruit picked with stepladders; fungus trees underground | Gathering zones over shrubs; seasonal trees (one tile) | No orchards, felling, underground wood |
| Fishing | Fish visible in water, cleaned at a fishery, stocks deplete | Fishing labor from deep water, capped at 20 fish, infinite stock; one legendary fish per site (`farming.rs`, `fishlore.rs`) | No depletion or fishery |
| Cooking and drink | Meals from several ingredients, named and valued by ingredients; drinks per plant | Brew drink, cook meals, millstone flour (`buildings.rs`) | No named dishes, recipes or vintages |
| Spoilage | Rot speeds with heat; vermin eat stored food | Corpses rot (`game/miasma.rs`) | Food never spoils or gets eaten by vermin |
| Livestock | Many species per civ; breeding needs contact; gelding; herd limits | Dogs, cats, goats; pastures; pregnancy and litters, cap 24 per species (`game/animals.rs`) | Few species, no gelding |
| Feeding | Grazers need grass or cave moss scaled to size; deserts starve herds | Grazers are just assigned to pastures (`assign_grazers`) | No animal hunger |
| Animal products | Wool, milk, cheese, eggs and nest boxes, honey and wax | None | Missing (Inbox) |
| Taming and training | Per-species training knowledge, decay, half-tamed beasts going wild; war and hunting animals | Cage-caught animals tamed; war dogs follow soldiers | No trainers or per-species knowledge |
| Pets | Pets adopt owners, eat vermin, fight | Cats adopt, dogs taken in, named, mischief, mourned (`game/pets.rs`) | No pet-on-pet violence, no ratting |
| Hunting | Hunters with ammo, small game, bringing kills home | Hunting inside the Provision ambition (`game/ambitions.rs`) | Thin |
| Vermin | Rats in stores, burrowers in fields, colonies | None | Missing |
| Wildlife populations | Wildlife numbers fall with hunting and slowly recover; industry angers savage land | Wildlife by biome spawns freely | No population, no backlash |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| AGR-01 | Pets eat pets | A hungry carnivorous pet kills another's pet; the owners come to blows and a grudge starts. | Partial: pets and grudges | S | ★★★ | B20 | Bloat#168; Bloat#95 |
| AGR-02 | Livestock raids | Hungry neighbours, bandits or kobolds rustle animals from outdoor pastures at night. | Missing | S | ★★★ | B20 | Story:The Nation of Man; Story:Night of the Kobold; Story:catalogue |
| AGR-03 | Soil depth | Surface soil far above the caverns farms poorly while cavern soil is rich, pulling farms (and farmers) into danger. | Partial: crops by place | S | ★★★ | B12 | Log2023 |
| AGR-04 | Vermin in stores and fields | Hoarded food draws rats and worse, which eat seeds and food left in piles; burrowers in fields; ratting cats earn their keep. | Planned: B12 (pests) | M | ★★★ | B20 | Bloat#11; Bloat#12; Req336; Log2024 (vermin raids); Rel40d (vermin colonies) |
| AGR-05 | Named drinks and vintages | Brews get names commemorating events; fine drink is bottled, labelled and aged; drinking rituals; collectors. | Missing | S | ★★★ | C11 | Bloat#109; Bloat#212 |
| AGR-06 | Signature recipes | A great cook's dish gets a name, joins the civ's recipes and turns up in later forts; nobles crave particular dishes. | Missing | M | ★★★ | C11 | Req129; Req162; Bloat85 (roadmap) |
| AGR-07 | Training knowledge per species | Each fort learns to tame a species by trying; training fades without practice and half-tamed beasts go wild and violent; only young beasts train fully. | Partial: cage taming | M | ★★★ | D09 | Rel0.40; Bloat#50 |
| AGR-08 | Flood farming | Spring floods or opened floodgates water and enrich fields beside rivers. | Planned: F05 (spring floods) | M | ★★★ | B12 | Bloat#339; Story:Steam |
| AGR-09 | Logging angers the wild | Felling trees on savage land brings packs of enraged beasts; elves notice too (ECO-05). | Missing | M | ★★★ | B20 | Log2025 |
| AGR-10 | Fish run out | Overfishing depletes a stretch of water with a warning; predators that ate those fish come closer. | Planned: Inbox fish stocks | S | ★★ | B20 | Rel0.31; Story:catalogue |
| AGR-11 | Grazing and hunger | Grazers eat grass or cave moss in their pasture, bigger beasts more; barren sites starve herds. | Planned: Inbox grazing | S | ★★ | D09 | Bloat#247; Rel0.34; Log2023; Livestock 2012-20 |
| AGR-12 | Eggs, nests and hatchlings | Fowl lay in nest boxes; hatchlings follow their mother; dragon and roc nests can be raided. | Planned: Inbox eggs | S | ★★ | D09 | Rel0.34; Log2025; Story:Mother of Death; Story:catalogue |
| AGR-13 | Beekeeping | A wild hive seeds artificial hives; honey and wax pressed from comb; mead; a hive limit. | Planned: Inbox bees | S | ★★ | D09 | Rel0.34; Log2023; Story:features |
| AGR-14 | Cats fish | Cats catch fish from ponds. | Missing | S | ★★ | B20 | Bloat#47 |
| AGR-15 | Animal messengers | Bats, birds or trained pets carry word to the mountainhome or allies, or report a death. | Missing | S | ★★ |  | Story:The Assassination of Zecalo Bronzeflower; Story:Animal Justice |
| AGR-16 | Wool, milk and cheese | Shearing and milking at the farm, cheese at a workshop, even milkable cave creatures. | Planned: Inbox milking, shearing | M | ★★ | D09 | Bloat#58; Rel0.34; 2011 Creature I.C; Story:features |
| AGR-17 | War beasts | Trained big beasts beyond dogs: armoured animals, beasts drawing chariots. | Partial: war dogs | M | ★★ | D09 | Story:Forest Befouled; Story:catalogue |
| AGR-18 | Wildlife populations | Local wildlife numbers fall with hunting, recover slowly, and predators hunt prey. | Missing | M | ★★ | B20 | Core78; Req80; Req561; 2012; Rel0.34 |
| AGR-19 | Visible fishing and big catches | Fish are seen in the water, fishing gets harder, large aquatic beasts can be hooked. | Partial: Fishing labor, F12 legendary catch | M | ★★ |  | Bloat#356; Bloat#81; Req326 |
| AGR-20 | Fishery | Fish are cleaned at a fishery before cooking; raw fish rots slowly in the pile. | Planned: Inbox fishery | S | ★ | C11 | Rel0.31; Bloat#225 |
| AGR-21 | Orchards and growths | Plants carry seasonal fruit, flowers and leaves; trees can be planted; gatherers use stepladders. | Planned: Inbox orchards | M | ★★ | B12 | Farming 2012-20; Rel0.40; Log2021 |
| AGR-22 | Underground wood | Grow trees or giant fungi underground; cavern spores spread into dug soil. | Planned: Inbox fungus forest | M | ★★ | B12 | Bloat#304; Rel0.40 |
| AGR-23 | More livestock species | A wider bestiary of domestic animals, including extinct beasts some civs keep. | Partial: dogs, cats, goats | M | ★★ |  | Log2026 |
| AGR-24 | Multi-tile trees and felling | Trees with trunks and branches fall away from the cutter and crush what is beneath. | Missing | L | ★★ | B12 | Rel0.40; Story:Home Tree |
| AGR-25 | Herd management | Breeding and culling rules, gelding, replacing lost handlers, herd caps. | Partial: pastures, butcher mark, cap 24 | S | ★ | D09 | Bloat#281; Req509; Req374; Rel0.40 |
| AGR-26 | Breeding needs contact | Animals only mate when kept close together. | Partial: pasture breeding | S | ★ | D09 | Rel0.31 |
| AGR-27 | Better hunters | Hunters carry small game, finish a kill when interrupted, watch their ammo and stay home when hurt. | Partial: Provision hunting | S | ★ |  | Bloat#13; Bloat#14; Req76; Req445 |
| AGR-28 | War animals aren't food | Making an animal a war beast unmarks it for slaughter. | Partial | S | ★ |  | Log2023 |
| AGR-29 | Taming costs | Taming uses food and is risky work (danger pay in ECO-53). | Missing | S | ★ |  | Rel0.40; Bloat#96 |
| AGR-30 | Animal trainers and cues | Trainers drill animals, which only perform well on their handler's signal. | Planned: Inbox trainers | S | ★ | D09 | Story:Wedding of Doom; Story:catalogue |
| AGR-31 | Spoilage by heat | Food and butchered goods rot faster in warm places and freeze in cold. | Partial: corpses rot | S | ★ | C11 | Req360; Log2025 |
| AGR-32 | Flooded plants die | Plants under water die; nothing grows under buildings. | Missing | S | ★ |  | Rel0.31 |
| AGR-33 | Peat | Peat as fuel and resource. | Missing | S | ★ |  | Bloat#337 |
| AGR-34 | Leftover trade vermin | Small animals sold off come back as pets or quietly vanish. | Missing | S | ★ |  | Bloat#125 |
| AGR-35 | Fitting beasts per people | Each people uses animals of its size (no full horses pulling dwarf wagons). | Partial: civ gear | S | ★ |  | Bloat#234 |
| AGR-36 | Moisture and irrigation | Fields need watering by bucket or channel. | Planned: B12 | M | ★ | B12 | Bloat#335; Farming 2012-20 |
| AGR-37 | Fertiliser and fallow | Potash fertilises; fields need rest; farmer skill raises yields. | Planned: B12 | M | ★ | B12 | Bloat#335; Log2007; Rel0.31 |
| AGR-38 | Pack animals and mounts | Pack beasts carry goods, mounts carry riders and the wounded. | Missing | M | ★ |  | Bloat#195; Log2024; Trader role; Story:catalogue |

*Dropped (adventure only): Bloat#160 eating live vermin, Bloat#327 riding positions.*

## Construction, engineering & fluids (ENG)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Constructions | Walls, floors, stairs, ramps, fortifications, roads, bridges, grates, bars, hatches, windows, supports | Wall, floor, stairs, ramp, fortification, remove; box-planned (`game/construct.rs`) | No roads, grates, bars, hatches, windows, supports |
| Doors and gates | Doors, hatches, drawbridges of any width over open space that fling what's on them; secret doors planned | Lockable doors, one-tile drawbridges, floodgates (`game/traps.rs`) | No wide/moat bridges, hatches, secret doors |
| Mechanisms | Levers and pressure plates (weight or fluid depth) linked by a mechanic who walks there | Levers linked instantly to doors, bridges, floodgates | No pressure plates; no install trip; no lever overview |
| Traps | Cage, stone-fall, weapon, upright spikes; planned hammer traps, rolling boulders, multi-tile blades | Cage, weapon (3 strikes), stone-fall (`game/traps.rs`) | No spikes, pits, plates; no jamming by bodies |
| Water | Flowing water with pressure, salt and stagnant water, evaporation, freezing, ponds, wells, cisterns | Flowing and levelling water, sleeping still water, wells over channels, floodgates, aquifers light/heavy, winter ice (`game/water.rs`) | No salt/stagnant water, ponds, cisterns, flow pushing |
| Magma | Magma flows, sets to obsidian, powers furnaces, pumpable, volcanoes refill | Magma sea flows, burns, kills, sets to obsidian; magma smelter and forge | No pumping, channels or refill |
| Pumps and power | Screw pumps by hand or power; water wheels, windmills, axles, gears, millstones, rollers | None | All of W26 open |
| Minecarts | Track, rollers, stops, routes (see ECO-29) | None | Missing (B11) |
| Structural support | Unsupported ground collapses; supports; dug-out ramps give way | Spans of 5+ from support collapse, creak at 9 (`game/disasters.rs`) | Span-only, no disconnection or supports |
| Fire | Spreads through grass, trees and wood; kindling; burning buildings | Spreads in trees, shrubs, grass, wood; put out by water and rain (`game/fire.rs`) | No bucket brigades, no deliberate burning |
| Light | Planned darkness, torches, lamps, gem windows | None | B13 open |
| Temperature | Heat from magma, freezing, steam | Ice and snow only (W19) | No temperature model |
| Rooms and zones | Rooms from furniture, many zone kinds, overlapping zones, bulk room creation | Zones make rooms; room value by size and furniture (`game/rooms.rs`) | One zone per tile; no bulk rooms |
| Grand works | Megaprojects of the player's own design | 3x3 monument with scaffolding and feast (`game/megaprojects.rs`); Architect's grand works | No machines or moving structures |
| Mess and cleaning | Spatter piles up, footprints, cleaning, refuse dumped in chasms | Dump zone; overseer's refuse pile | No spatter or cleaning |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| ENG-01 | Traps jammed with bodies | Corpses and carcasses pile up in a trap corridor until it stops working and someone must clear it, maybe under fire. | Missing | S | ★★★ | O04 | Story:Animal Justice; Story:Half Dwarf |
| ENG-02 | Deliberate flooding | The overseer opens floodgates to drown invaders or escaping prisoners in a sealed section. | Partial: lever floodgates exist, never used this way | S | ★★★ | O04 | Story:Summoned to Darkness; Story:Steam |
| ENG-03 | Pits and ponds | A zone over a hole lets haulers fill it with water bucket by bucket, or drop captured creatures and prisoners into it. | Missing | S | ★★★ | O04 | Rel0.31; Log2022 |
| ENG-04 | Upright spikes | Lever-raised spikes, and spiked pits that impale whatever falls in. | Missing | S | ★★★ | O04 | Rel0.34; Story:Bride Price |
| ENG-05 | Fire as tool and threat | Designate things to burn, kindling and hay, burning buildings to flee, bucket brigades from the well. | Partial: fire spreads; Inbox bucket brigades | S | ★★★ | B13 | Fire 2012-20; Core14 |
| ENG-06 | Pressure plates | Triggered by creature weight or fluid depth (even "empty"), linked to doors, bridges and traps, outdoors too. | Planned: Inbox pressure plates | S | ★★ | O04 | Bloat#29; Log2007; Log2022 |
| ENG-07 | Trapdoors and chutes | Hatches and slides that drop whoever stands on them into a cell below. | Planned: Inbox hatches | S | ★★ | O04 | Story:catalogue; Log2023; Log2007 |
| ENG-08 | Nets and tripwires | Tripwire gauntlets and nets that snare or trip intruders. | Missing | S | ★★ | O04 | Story:Half Dwarf; Story:Curse of the Cyclops |
| ENG-09 | One-column stairs | Designate a single column and it becomes up stair at the bottom, down stair at top, both between; stairs into open air; multi-level designations. | Partial: constructed stairs | S | ★★ | O11 | Log2022; Rel0.34 |
| ENG-10 | Builders don't wall themselves in | Builders pick where to stand; diggers start from the nearest tile. | Partial | S | ★★ | O11 | Rel0.34 |
| ENG-11 | Chasm dumping | Refuse, corpses or any marked object are thrown into a chasm. | Partial: dump zone, chasm site feature | S | ★★ |  | Req28; Req47; Rel0.31 |
| ENG-12 | Magma melts constructions | Magma wears and melts constructions of meltable material. | Partial | S | ★★ |  | Req554 |
| ENG-13 | Flow pushes | Fast water and magma shove creatures and items along. | Missing | S | ★★ |  | Rel0.31 |
| ENG-14 | Ice and thaw | Freezing traps creatures in ice; thin ice breaks; spilled water evaporates; rain pools; deep snow slows. | Partial: ice and snow (W19) | S | ★★ |  | Log2007; Bloat286; Bloat287; Bloat292 (roadmap) |
| ENG-15 | Warm and damp warnings, aquifer tools | Digging near magma or water warns; a shaft can be pumped dry; warning before embarking on a heavy aquifer. | Planned: W26 (aquifer step) | S | ★★ | W26 | Log2022; Log2019 |
| ENG-16 | Named halls | Custom names for buildings, halls and stockpiles. | Planned: O03 | S | ★★ | O11 | Req421 |
| ENG-17 | Bulk rooms and overlapping zones | One drag over many enclosed rooms makes each its own room; zones repaint, move and overlap. | Partial: zones; Inbox overlapping zones | S | ★★ | O11 | Log2022 |
| ENG-18 | Wells and cisterns inside | A cistern fed by a channel under the halls, with a well above it. | Planned: S05 (last step) | S | ★★ | W26 | Bloat#360; Story:Steam |
| ENG-19 | Better traps | Hammer traps that fling, rolling boulders down ramps, stone-falls that need a stone set above, multi-tile blade halls with a spring to reset, traps that keep striking. | Planned: B07 | M | ★★★ | O04 | Bloat#278; Bloat#295; Bloat#372; Improved mechanics |
| ENG-20 | Moats and wide drawbridges | Drawbridges of any width over open space that fling what's on them when raised. | Planned: Inbox drawbridges, O04 | M | ★★★ | O04 | Bloat#28 (great gates) |
| ENG-21 | Screw pumps | Lift water or magma one level by hand or power; stacks drain shafts. | Planned: W26 | M | ★★★ | W26 | Log2007 |
| ENG-22 | Size-limited passages | Narrow doors stop large creatures (who may smash through); cramped tunnels hamper archers. | Missing | M | ★★★ | D04 | Story:Home Tree; Story:Orther Pushes His Luck |
| ENG-23 | Secret doors and escape tunnels | Hidden or one-way doors; passages for evacuation, sorties or smuggling. | Missing | M | ★★★ | D04 | Bloat#280; Story:Half Dwarf; Story:Summoned to Darkness |
| ENG-24 | Supports and collapse | Ground cut off from support falls; supports to place and pull; trees and towers topple onto what's below; sabotage by removing a support. | Partial: span cave-ins | M | ★★★ | D04 | Req459; Req506; Req507; Log2006; Story:Dwarven Servitude; Story:Home Tree |
| ENG-25 | Light and darkness | Dark caverns and nights, torches, candles and lamps, coloured light through gem windows. | Planned: B13 | M | ★★★ | B13 | Req96; Core55; Arc Fire and Lighting |
| ENG-26 | Eruptions and surges | Volcanoes or the magma sea surge and flood low levels. | Planned: F05 | M | ★★★ | W26 | Story:Summoned to Darkness; Story:catalogue |
| ENG-27 | Magma works | Channels carry magma to forges or moats; magma poured on attackers. | Planned: W26, O04 | M | ★★★ | W26 | Log2007; Story:Steam |
| ENG-28 | Power | Water wheels, windmills facing the wind, axles and gears drive pumps, millstones and rollers; power summed over a network. | Planned: W26 | L | ★★ | W26 | Bloat#350; Req541; Log2007; Log2022 |
| ENG-29 | Architecture with meaning | Ideas tied to architectural elements in towers, castles and halls, not only temples. | Partial: Architect grand works | M | ★★ |  | Bloat#381 |
| ENG-30 | Moving sections | Lifts, crushing ceilings and moving floors. | Missing | L | ★★ | D04 | Improved mechanics |
| ENG-31 | New liquids | Mud, oil, blood, slime, sand, even a beer fountain. | Missing | L | ★★ |  | Bloat#325 |
| ENG-32 | Great gates | Huge gates opened from afar. | Partial: lever drawbridge | S | ★ |  | Bloat#28 |
| ENG-33 | Better levers | A lever lists its targets, can be unlinked, toggles or times; a mechanic walks over to install the link. | Partial: instant linking; Inbox | S | ★ | O11 | Bloat#279; Req37; Log2022 |
| ENG-34 | Grates, bars and windows | Grates pass water and small items but not creatures; bars stop bodies but not shots; glass and gem windows. | Planned: Inbox | S | ★ |  | Bloat#370; Log2007; Log2023 |
| ENG-35 | Roads and wagon access | Roads between buildings; wagons need a three-wide path to the depot, shown on the map. | Planned: Inbox roads | S | ★ | O11 | Bloat#293; Req313; Req422; Rel40d |
| ENG-36 | Bridges, rafts, lake platforms | Cross rivers by bridge or raft; build platforms and buildings on underground lakes. | Partial: floors bridge gaps | S | ★ |  | Story:The Adventures of Gornon; Bloat#190 |
| ENG-37 | Fresh, salt and stagnant water | Murky or salt water upsets drinkers; fresh water clears stagnant. | Partial: river vs well; Inbox murky water | S | ★ | W26 | Rel0.31 |
| ENG-38 | More water sources | Rain barrels, filling barrels at streams, desalination, salt ponds; pools refill with rain. | Partial: wells, rivers | S | ★ | W26 | Bloat#360; Req524; Rel0.31 |
| ENG-39 | Building in water | Drop simple constructions into water; fill channels back in. | Partial | S | ★ |  | Req75; Req583 |
| ENG-40 | Volcanoes refill | A drained magma pipe slowly refills. | Missing | S | ★ | W26 | Rel0.31 |
| ENG-41 | Floating | Light items and swimmers float. | Missing | S | ★ |  | Req433 |
| ENG-42 | Burying things | Bury items in soil to hide them from thieves. | Missing | S | ★ |  | Req99 |
| ENG-43 | Spatter and cleaning | Blood, mud and vomit pile up by amount, stain and dry; footprints; rain washes; cleaning designations. | Missing | S | ★ |  | Req289; Bloat#170; Bloat#319; Bloat#321; Req462; Rel40d |
| ENG-44 | Clearing stuck rubbish | Get refuse out of fortifications and unreachable tiles. | Missing | S | ★ |  | Bloat#294 |
| ENG-45 | Build over built floors | Any construction can go on a built floor without removing it first. | Partial | S | ★ | O11 | Log2022 |
| ENG-46 | Weather-tight buildings | Roofed buildings keep off rain and snow. | Partial: roof makes a room | S | ★ |  | Log2023 |
| ENG-47 | Workshop footprints | Fix trouble from blocked tiles in multi-tile workshops. | Partial | S | ★ |  | Bloat#301 |
| ENG-48 | Box construction and map notes | Lay many constructions in a box; pin notes and waypoints to tiles. | Partial: box constructions | S | ★ |  | Rel0.34 |
| ENG-49 | Mining options | Dig without leaving stone; dig out of a pit you're stuck in. | Missing | S | ★ |  | Req74; Req552 |
| ENG-50 | Hearths and chimneys | Indoor fires need a smoke outlet; warmth matters on cold sites. | Planned: Inbox temperature | M | ★ | B13 | Story:Cold Fire; Story:catalogue |
| ENG-51 | Wear on stonework | Old masonry and engravings wear; loose bricks open escape routes. | Missing | M | ★ | D04 | Story:The Adventures of Gornon; Story:catalogue |
| ENG-52 | Fluid heat | Warm water carries heat; steam scalds. | Missing | M | ★ | B13 | Req580; Req457 |
| ENG-53 | Finer pressure | Fluid pressure at finer resolution. | Partial: water falls and levels | M | ★ |  | Bloat#331 |
| ENG-54 | Pipes | Pipes you can walk on or crawl through that carry fluids over many tiles. | Missing | L | ★ |  | Bloat#316 |

*Dropped: Bloat#226 web check, Bloat#284 river rocks and Bloat#334 river flow look (engine/visual trivia; listed under Other in bloat.md).*

