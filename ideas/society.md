# Society: family, culture, nobles and justice

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## Society, family & culture (SOC)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Relationships | Friends, grudges, lovers, war buddies, childhood friends, athletic rivals; trust, loyalty, fear and respect dials decide who can be turned or who talks | One signed score per pair from chatting; friend/grudge (`psyche.rs` relations) | Only one dial; no bond types, no history of how a bond formed |
| Romance & marriage | Courtship over time, affairs, multiple lovers, children before marriage, divorce, rejection, jealousy | Close friends marry; rivals in love grow jealous (`family.rs`, `drama.rs`, F10) | No courtship stages, ceremony, affairs or divorce |
| Family structure | Parents, siblings, grandparents, heirs; orphans adopted; mothers nurse and hunt for lost infants | Sex, age, pregnancy, children with the mother's family name, parents tracked (`family.rs`) | No siblings/grandparents, nursing, adoption, inheritance |
| Children | Play with toys, follow mother, do chores, grow up into adults with inherited traits | Children play and become adults at 12 (`family.rs`) | No toys, chores, rites, heredity, rebellious streak |
| Feuds and violence | Insults escalate slowly; brawls draw in kin and friends; pets cause feuds; a breaker ends lethal fights | Pranks, insults, duels at dawn to a yield, Unbeaten title (`drama.rs`, F10) | One-on-one only; no factions, shunning, pet feuds |
| Festivals & parties | Civ-value festivals with races and processions; private parties with guest lists; official feasts by nobles | Founding, harvest, midwinter, dedication; dance, drink, arm-wrestling, drinking contest (`festivals.rs`, F03) | No private parties, weddings, anniversaries of fort events, etiquette |
| Taverns & performance | Keepers serve drinks; performers play generated instruments, dance and recite generated art forms; drunks get refused | Visiting bards sing songs built from legends (`visitors.rs`) | No keepers, performers on staff, instruments, drunkenness |
| Visitors & residency | Scholars, mercenaries, performers lodge, petition to stay, become residents then citizens; old enemies meet | Bards and wandering warriors visit; warriors may join (`visitors.rs`) | No residency step, no rented rooms, no feuds between guests |
| Migration | Migrants are real historical figures with kin and comrades; families follow; losses scare migrants off; waves can be refused | Waves suppressed by reputation spread by merchants and tappers (`game.rs`, `word.rs`) | Migrants are anonymous; no baggage, refusal, or returning survivors |
| Rumour & reputation | News lives in heads, travels by talk; liars and tall tales; killing witnesses buries a deed | Gossip tab; word gets around the world about what you lack (`word.rs`, `drama.rs`) | No per-dwarf knowledge of events, no lying |
| Subgroups | Guilds and religious congregations petition for halls, hold demonstrations, take sides in grudges | None | B09 open |
| Knowledge & books | Scholars research, write books, libraries hold them; books spread values and locate ruins | Generated names only (`word.rs` naming); legends screen | B08 open |
| Games & gambling | Dice, cards, board games, betting, arenas with chained beasts | Arm-wrestling and drinking contests at festivals | No betting, debts, arena |
| Culture & dress | Each culture has its own clothes, dyes, calendar, customs, art forms | One culture's festivals; no dress variety | Culture is flavour-thin |
| Death & memory | Grief by tie, anniversaries, heroes remembered | Grief weighted by spouse/parent/child; heroes mourned; memorial slabs; ghosts (`psyche.rs`) | Mostly there; no anniversaries or naming children after the dead |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| SOC-01 | Named after heroes | A newborn takes the name of a fort hero or famous figure; the parents' reason is logged in the chronicle. | Have: done in C05 (WISHLIST) | S | ★★★ | C05 | Bloat#183, roadmap Society |
| SOC-02 | Weddings as events | Marriage triggers a short ceremony and feast with gifts; a jealous rival or enemy may turn up uninvited. | Have: done in C05 (WISHLIST) | S | ★★★ | C05 | Story:Weddings, Bloat#93, Core59 |
| SOC-03 | Shunning | A dwarf with crimes or many grudges is cold-shouldered: others refuse to chat, stress rises, they drift to the edges of the fort. | Missing | S | ★★★ | C03 | Bloat#105 |
| SOC-04 | Factions in a brawl | When a fight breaks out, onlookers join by friendship and kin or try to break it up; the split leaves lasting grudges between camps. | Partial: duels one-on-one (F10) | M | ★★★ | C03 | Story:Factions in a brawl |
| SOC-05 | Drunks and bar brawls | Tolerance varies by dwarf; drunks stumble, start fights, pass out and are carried to bed; keepers refuse the visibly drunk; drinking at the wrong time is a crime. | Partial: drinking need and contests | S | ★★★ | C09 | Bloat#373, Arc Crime, PG38, PG175, Log2020 drinking-to-death, Story:Drunkenness |
| SOC-06 | Gambling and debt | Dice, cards and board games in the tavern; bets on duels and contests; heavy losers owe creditors and grudges follow. | Missing | S | ★★★ | C09 | Bloat#36, 2016 taverns, PG42-43, Story:Betting and debt |
| SOC-07 | Grudges from pets | A cat that kills a dwarf's pet, or a dog that bites, starts a feud between owners. | Partial: pets mourned, grudges exist | S | ★★★ | C03 | Bloat#95 |
| SOC-08 | Slow road to violence | Offence escalates in stages (shouting, lying, being found out) before blows; each step is visible in the Gossip tab. | Partial: insult -> duel (F10) | M | ★★★ | C03 | Bloat#207 |
| SOC-09 | Living calendar of anniversaries | Big fort events (first siege, a hero's death, an artifact) become yearly remembrance days with their own small festival. | Partial: founding day | S | ★★★ |  | Bloat#112 |
| SOC-10 | Nicknames for anyone | Mishaps, looks and crimes earn mocking nicknames alongside heroic epithets ("Twice-Drowned"). | Partial: heroic epithets only | S | ★★★ | C03 | Story:Nicknames for anyone |
| SOC-11 | Seasonal champion | Festival contests crown a champion who holds the title until the next festival; losing it stings. | Partial: contests and Unbeaten epithet | S | ★★★ |  | Story:Seasonal champion |
| SOC-12 | Coming-of-age rite | At twelve a child faces a trial or first task; failure brings shame, success a name. | Missing | S | ★★★ | C05 | Story:Coming-of-age rites, PG40, PG169, PG170 |
| SOC-13 | Orphans adopted | A child whose parents die is taken in by a kind dwarf (a friend of the dead), forming a new family tie. | Missing | S | ★★★ | C05 | Req137 |
| SOC-14 | Visitors bring old feuds | Guests don't know who else lodges in the tavern; old enemies meet and fights or murders follow. | Missing | S | ★★★ | C09 | LogOld visitors |
| SOC-15 | Outsider suspicion | Newcomers with odd habits are distrusted and blamed first when something goes missing or someone dies. | Missing | S | ★★★ | C03 | Story:Outsider suspicion |
| SOC-16 | Migrants with a past | Migrants are drawn from world-history figures and arrive with their old comrades, kin and enemies; relatives follow later. | Partial: anonymous migrants | M | ★★★ | B19 | LogOld migrants, Log2020 migrants with a past, Core77 |
| SOC-17 | Survivors of fallen forts return | Dwarves who escaped one of your earlier, abandoned forts in this world turn up as migrants with memories of it. | Missing | S | ★★★ | B19 | LogOld fallen forts |
| SOC-18 | Death drives migrants away | Heavy losses (above all dead nobles) thin the next wave, and the game says why. | Partial: reputation via `word.rs` | S | ★★★ | B19 | LogOld migrants |
| SOC-19 | Tall tales and liars | Dwarves embellish their exploits in the tavern; listeners' opinions shift; known liars get a reputation in the legends. | Partial: bards sing legends | S | ★★★ | C09 | Bloat#192, Bloat#196, PG110, Story:Tall tales |
| SOC-20 | Richer romance | Courtship takes time; affairs, rejection, breakups and divorce each leave thoughts and grudges. | Partial: marriage, jealous rivals | M | ★★★ |  | Log2020 richer relationships, LogOld gradual romance, Bloat#93, Core59 |
| SOC-21 | Forbidden love | Parents or a noble oppose a match; the pair meet in secret and the scandal leaks a few dwarves at a time. | Missing | M | ★★★ |  | Story:Forbidden love |
| SOC-22 | Rebellious youth | A child defies its parents: sneaks into the barracks to train, befriends a parent's enemy. | Missing | M | ★★★ | C05 | Story:Rebellious youth |
| SOC-23 | Sibling rivalry and favourites | Parents favour one child; siblings compete, resent or defend each other. | Missing | S | ★★ | C05 | Story:Sibling rivalry |
| SOC-24 | Siblings, grandparents, heirs | Track extended family; kin react to each other's harm and death; migrant siblings know each other. | Planned: Inbox sibling/grandparent ties | M | ★★ | C05 | Core60, Bloat#123, WISHLIST Inbox |
| SOC-25 | Mothers and babies | Babies are nursed and carried; children follow their mother; a mother hunts for a lost infant and reacts when it is hurt. | Planned: Inbox nursing | S | ★★ | C05 | Req391, Req406, Req213, Log2020 mothers, LogOld children follow mother |
| SOC-26 | Arena and fight pit | Captured beasts or goblins fight champions before a crowd that wants a show; a bookmaker takes bets. | Partial: Arena mode on the title screen, not in the fort (C09) | M | ★★★ | C09 | Bloat#233, PG41, PG67, Story:Arena and fight pit |
| SOC-27 | Better parties | A dwarf throws a party for a friend or family; nobles hold official feasts with guests ranked by status; the annoying cousin comes too. | Partial: F03 festivals | S | ★★ |  | Bloat#92, PG156 |
| SOC-28 | Feast etiquette | Seats of honour, announced arrivals and toasts at feasts; a breach is laughed at or punished by the host noble. | Missing | S | ★★ |  | Story:Feast etiquette |
| SOC-29 | Music at gatherings | Dwarves play instruments at festivals and parties; playing well earns praise. | Partial: bards sing | S | ★★ |  | Bloat#94, Log2020 musical instruments |
| SOC-30 | Tavern keepers and performers | A keeper serves drinks; resident performers dance, play and tell legend stories; the tavern's liveliness meets needs. | Partial: bards on the stage, dwarves play instruments, 1ff31da; no keeper (C09) | M | ★★★ | C09 | Log2020 tavern keepers, 2016 taverns, Story:Generated poetry |
| SOC-31 | Taverns with paying guests | Merchants, mercenaries and adventurers rent rooms; the fort earns a reputation as an inn; trouble feeds justice. | Planned: B08 | M | ★★★ | C09 | 2016 taverns, 2020 |
| SOC-32 | Residents and citizens | Long-staying visitors petition to stay, then to become citizens; standing orders accept or refuse them. | Partial: wanderers may join | M | ★★ | C09 | Log2020 residents vs citizens, LogOld residency |
| SOC-33 | Guilds and sects | Enough crafters or believers form a group that petitions for a guildhall or temple, takes sides in grudges and grants perks. | Planned: B09 | M | ★★★ | B09 | Bloat#62, Bloat#305, 2012-2020 subgroups, Log2020 petitions, Story:Clubs and orders |
| SOC-34 | Guild demonstrations | Members gather for skill demonstrations that teach and meet needs. | Planned: B09 | S | ★★ | B09 | Log2020 guild demonstrations |
| SOC-35 | Masters and apprentices | A veteran takes an apprentice, teaches, scolds and favours one, provoking envy. | Partial: workshop masters and apprentices, ECO-13 (O08); no favourites or envy | M | ★★★ | B09 | Story:Masters and apprentices |
| SOC-36 | Childhood friends and war buddies | Bonds formed growing up or fighting together are stronger; performances and contests forge friendships or rivalries. | Partial: friendships | S | ★★ |  | LogOld war buddies, Log2020 richer relationships |
| SOC-37 | Founders' journey bonds | The starting seven arrive already friends (or one quarrel) from the trip. | Missing | S | ★★ |  | LogOld founders |
| SOC-38 | Relationship dials | Trust, loyalty, fear and respect alongside love/hate decide who confides, who can be turned, who informs. | Partial: one score | M | ★★ | C03 | LogOld five dials, Log2020 trust/respect |
| SOC-39 | Children at play | Kids form cliques, bully, play board games and reenact the fort's own legends with toy weapons. | Partial: children play | S | ★★ |  | Bloat#3, Bloat#41, Bloat#171, Story:Children at play |
| SOC-40 | Children's chores | Children fetch, haul light items and help in workshops. | Missing | S | ★★ |  | Log2020 children's chores |
| SOC-41 | Hospitality | Strangers are fed and housed or turned away by culture and first impression; servants tidy before an important guest. | Partial: visitors | S | ★★ |  | Story:Hospitality, PG57 |
| SOC-42 | Culture shock | Visitors and migrants judge the fort by their own culture (an elf horrified by stumps and leather). | Missing | S | ★★ |  | Story:Culture shock |
| SOC-43 | Rumours in heads | News lives in individual dwarves and spreads by talk; killing every witness buries a deed; the liaison brings world news. | Partial: Gossip tab, `word.rs` | M | ★★★ |  | LogOld rumours, 2020 plots |
| SOC-44 | Turning migrants away | The overseer can refuse a wave, at a cost to standing back home. | Missing | S | ★★ | B19 | Bloat#312, roadmap refusing migrants |
| SOC-45 | Ask for the workers you need | Request particular trades from the homeland through the liaison. | Missing | S | ★★ | B19 | Req451 |
| SOC-46 | Persecuted kin | Dwarves fleeing a purge in a human city arrive as refugees, carrying stories and pursuers. | Missing | M | ★★★ | B19 | Story:Persecuted kin |
| SOC-47 | Coming home | Expedition members or long-gone dwarves react to family and changes when they return. | Missing | S | ★★ |  | Story:Coming home |
| SOC-48 | Altruism | Dwarves help drunks to bed, lift the fallen, comfort crying children; kindness builds friendships. | Missing | S | ★★ |  | Story:Altruism |
| SOC-49 | Beauty and superlatives | The fort tracks its most beautiful, strongest and most hated dwarf; people glance at the striking. | Missing | S | ★★ |  | Story:Beauty and superlatives |
| SOC-50 | Crowds | Gathered dwarves cheer, jeer or panic together; moods spread through a crowd. | Missing | M | ★★ |  | Story:Crowds |
| SOC-51 | Inheritance | A dead dwarf's goods and room pass to kin by custom; passed-over children resent it. | Missing | M | ★★ | C05 | Story:Inheritance, Core60 |
| SOC-52 | Heredity | Children inherit personality facets and looks from parents; families come to look alike. | Missing | M | ★★ | C05 | Bloat#347, Story:Inherited traits |
| SOC-53 | Migrants with baggage | Migrants bring infants, small pets, pack animals and goods suited to their trade. | Missing | S | ★ | B19 | Bloat#124, Bloat#309 |
| SOC-54 | Libraries and scholars | Dwarves write books on history and skills; libraries hold them; visiting scholars read and debate; a book can locate a ruin. | Planned: B08 | M | ★★ | B09 | Bloat#185, Core100, 2016, LogOld scholars, Log2020 libraries |
| SOC-55 | Books spread values | Philosophy books and poems shift the values of readers. | Missing | M | ★★ | B09 | LogOld books spread values |
| SOC-56 | Generated art forms | Each civ has its own poetry, music and dance forms, taught teacher to student and performed. | Partial: bards' songs | L | ★★ |  | LogOld art forms, 2016 taverns, Story:Generated poetry |
| SOC-57 | Generated instruments | Multi-part instruments of culture-specific materials, crafted and played. | Have: arts.rs, each people's instruments made from the world's seed (names, materials, how they're played), crafted and played in the fort (culture.rs) | S | ★ |  | LogOld instruments, Log2020 instruments |
| SOC-58 | Costumes and masks | Craftable costumes and masks worn at festivals. | Missing | S | ★ |  | Bloat#37, Bloat#38 |
| SOC-59 | Fashion and dress | Clothing by culture, favoured dyes and materials; dwarves comment on outfits. | Planned: Inbox dyes/clothing | M | ★ |  | Bloat#242-243, Bloat#264 |
| SOC-60 | Smoking circles | Pipes and pipe-weed shared in circles, a social need. | Missing | S | ★ |  | Bloat#43 |
| SOC-61 | Customs and deviations | Small rituals (toasts, knocking); breaking them raises eyebrows. | Missing | M | ★ |  | Story:Customs and deviations |
| SOC-62 | Art history | Commentary on an artist's career periods in the legends. | Missing | M | ★★ |  | Bloat#22 |
| SOC-63 | Lost and found knowledge | Civs know techniques to a degree; knowledge spreads, is lost and rediscovered; breakthroughs happen. | Missing | L | ★★ |  | Bloat#27, 2016 innovation |
| SOC-64 | Writing and grammar | Generated languages with grammar; writing on walls and blades. | Partial: `word.rs` names | M | ★ |  | Core97-99, Bloat#348 |
| SOC-65 | Writing materials | Paper, papyrus and parchment, quires and scrolls bound into books. | Missing | M | ★ |  | LogOld writing materials |
| SOC-66 | Faith passes on | Babies take their parents' gods; immigrants keep their own. | Planned: B14 | S | ★ |  | Log2020 faith passes on |
| SOC-67 | Mixed heritage | Children of mixed parentage favour one side and may face prejudice. | Missing | L | ★ |  | Story:Mixed-heritage dwarves |
| SOC-68 | Social clumping | Idle dwarves seek company in meeting zones; more chat types meet more needs. | Partial: meeting areas | S | ★ |  | Log2020 social clumping |
| SOC-69 | Family tree screen | One screen of relatives, spouses, friends and pets, with zoom-to. | Partial: family on dwarf screen | S | ★★ |  | Bloat#51, LogOld relationship screen |
| SOC-70 | Rename anything | The player names dwarves, squads and zones. | Missing | S | ★ |  | LogOld nicknames |
| SOC-71 | Age caps | Settings cap babies and children so a fort doesn't fill with infants. | Partial: births | S | ★ |  | LogOld population caps |

*Dropped as outside this domain or not fortress-relevant: adventure-mode conversation trivia; the fix-list entry on migrant history bugs (Bloat#342).*

## Nobles, justice & politics (NOB)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Noble ladder | Leader -> elected mayor -> baron/count/duke; baron may be chosen or refused; monarch may move in and the fort can become a Mountainhome | Leader, mayor at 20 pop, baron/count/duke by wealth+pop (`dwarf.rs`, `psyche.rs`) | No monarch, no choice/refusal, no succession drama |
| Noble demands | Room demands, item mandates, export bans, artifact demands, dish fussiness, office envy | One mandate at a time; furniture-of-material demands; room expectations (`psyche.rs`, `positions.rs`) | Overseer ignores mandates; no export bans, no favouritism |
| Appointed posts | Manager, bookkeeper, broker, medic, sheriff, hammerer, tax collector, champion | Six posts ranked by fit, effects scale (`positions.rs`) | No hammerer, tax collector, champion, symbols of office |
| Crimes recorded | Theft, murder, assault, vandalism, disobedience, conspiracy | Assault, vandalism, disobedience (`justice.rs`) | No theft or murder as citizen crimes |
| Investigation | Witnesses report, missing items noticed, unsolved cases, accusations (wrong ones too), interrogation reveals chains | Culprit is always known | No witnesses, no uncertainty |
| Punishment | Beatings, jail with chains/cages, prisoners fed, hammering, execution; armour stripped | Sheriff beats or jails 3 days per charge; victims gladdened (`justice.rs`) | No graded punishments, no executions, no banishment |
| Jail | Terms, escapes, overcrowding, prisoners fed by others | Jail zone, days inside, no work | No furniture, feeding, unrest |
| Corruption | Officials embezzle and take bribes over time, open to blackmail | None | Missing |
| Villains & infiltrators | World villains run agent chains; disguised agents turn dwarves to steal artifacts; vampires hide as citizens; spies | Skulkers steal from outside (`creature.rs`); hooded stranger may be a goblin spy (`mysteries.rs`, F16) | B04 open |
| Diplomacy at the fort | Envoys and liaison negotiate trade, tribute, peace; elves judge forestry; parley before sieges | None; wars and peace exist only in world history | B02 open |
| Succession | Dead office-holders replaced, contested claims, captured leaders force elections | Mayor elections; rulers succeed in worldgen only | No in-play succession or claimants |
| Order & unrest | Martial law, prison revolts, food riots, witch hunts | Tantrums and melancholy | No collective unrest |
| Overseer vs nobles | (DF has no AI overseer; the player is pressured by mandates, complaints and demands) | Brain has an Unrest sense; no noble agenda in the brain (`overseer.rs`) | Nobles never push the overseer |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| NOB-01 | Overseer answers mandates | Each mandate becomes a trouble and work order the overseer weighs against its plans by trait (Stubborn ignores, Vain overdelivers); an unmet one blames someone. | Partial: mandates issued, overseer never reads them | S | ★★★ | C06 | Bloat#250, Bloat#53 |
| NOB-02 | The council lobbies the overseer | Nobles and post-holders push agendas as a brain sense; a weak-willed overseer is steered, a Stubborn one defies them and pays in unrest. | Missing | M | ★★★ | C06 | Story:The council lobbies the overseer |
| NOB-03 | Banishment and expulsion | A sheriff or noble exiles a criminal, or the fort expels a resident; close kin leave too, friends resent it, exiles never return as migrants. | Planned: Inbox banishment | S | ★★★ | C07 | Bloat#143, PG47, LogOld expel, Log2020 expulsion, Story:Banishment, WISHLIST Inbox |
| NOB-04 | Graded punishments | Fines, beard-shaving, branding, the stocks, whipping instead of only beatings and jail; armour is removed before a beating. | Partial: beatings and jail | S | ★★★ | C07 | Bloat#75, Req511, Core83, 2012-2020 justice lists |
| NOB-05 | Witnesses report crimes | A dwarf who sees a crime or finds a body walks to the sheriff to report it (unless the sheriff did it). | Partial: crime record, culprit always known | S | ★★★ | C07 | LogOld witnesses |
| NOB-06 | Missing, then dead | A dwarf who dies alone is first only missing; death is confirmed by finding the body, a witness, or time. | Missing | S | ★★★ | C07 | LogOld missing |
| NOB-07 | Nobles play favourites | A noble mandates work that helps a friend or bans work by a rival's craft; the slighted hold grudges against the noble. | Missing | S | ★★★ | C06 | Bloat#78 |
| NOB-08 | Noble deaths matter | A noble's death angers kin and other nobles, chills migration, and a grand burial may seal servants in the tomb. | Missing | S | ★★★ | P02 | Req291, LogOld migrants |
| NOB-09 | Spin | A Vain overseer or noble writes a defeat up as a victory in the chronicle; dwarves who were there don't believe it. | Missing | S | ★★★ | C06 | Story:Spin |
| NOB-10 | Bribery | A culprit bribes the sheriff or guard to drop charges; some can't be bought. | Missing | S | ★★★ | C07 | Story:Bribery |
| NOB-11 | Choose your baron, or refuse | When the barony comes the overseer picks a dwarf (by its traits) or declines and loses trade goodwill. | Partial: civ names nobles automatically | S | ★★★ | C06 | LogOld choose baron |
| NOB-12 | Treating envoys badly | Imprison, cheat or kill an envoy or merchant; the civ remembers, and Reckless overseers might. | Missing (B02 base needed) | S | ★★★ | B02 | Bloat#31, roadmap abusing envoys |
| NOB-13 | Elves judge your forestry | Elven envoys weigh trees felled and wooden goods; offence sours trade or brings war. | Planned: B02 | S | ★★★ | B02 | Bloat#72, Req132, Req463, Req516 |
| NOB-14 | Prison unrest | An overcrowded jail revolts against the nobles; nobles stop jailing once cells are full. | Missing | S | ★★★ | C07 | Bloat#177, roadmap prison unrest |
| NOB-15 | Envoys with demands | Envoys arrive asking tribute, an artifact or a prisoner, or offering peace and trade; paying or refusing moves the next siege. | Planned: B02 | M | ★★★ | B02 | B02, Core62, Arc Diplomacy, Log2020 diplomacy on your schedule |
| NOB-16 | Parley at the gate | A besieging army halts and sends a herald demanding an artifact, noble or criminal; handing it over buys years of peace. | Planned: B02, B05 | M | ★★★ | B02 | LogOld parley, Log2020 artifact bribe, 2020 artifacts, 2025, Story:Ransom demands |
| NOB-17 | Tribute and taxes to the homeland | The Mountainhome takes a yearly cut by caravan; refusing has consequences; a tax collector with guard escort. | Planned: B02 | M | ★★★ | B02 | Core81, Story:Tribute and taxes, LogOld tax collector, 2016-2020 |
| NOB-18 | Food riots | When the hungry see full noble larders, or a lord taxes food away, they riot and loot; guards crush it or don't. | Partial: hunger and low faith in the overseer set up to three stressed dwarves rioting, reign.rs `food_riot`; no looting, no noble larders, no guards (P02) | M | ★★★ | P02 | PG148, Story:Food riots |
| NOB-19 | Unsolved crimes and accusations | With no known culprit, the sheriff (or overseer) accuses a suspect from witness reports; grudge-holders and the guilty lie. | Planned: Inbox | M | ★★★ | C07 | LogOld accusations, WISHLIST Inbox theft |
| NOB-20 | Theft as a citizen crime | Citizens and visitors steal from stockpiles, the depot or each other; missing items are noticed; totals shown. | Partial: outside skulkers only | M | ★★★ | C07 | Bloat#113, Bloat#107, 2020 plots, Thief role |
| NOB-21 | Interrogation | Suspects or caged goblins are questioned with intimidation, persuasion or flattery; odds from personality and ties; a right arrest exposes the next link. | Partial: a caged besieger is questioned and lets slip the next siege (B04) | M | ★★★ | B04 | LogOld interrogation, Story:Interrogation, WISHLIST Inbox caged goblins |
| NOB-22 | Witch hunts and scapegoats | After unexplained deaths a mob or noble blames an outsider or misfit and punishes them, rightly or not. | Missing | M | ★★★ | C07 | Story:Witch hunts |
| NOB-23 | Corrupt officials | Post-holders face repeated temptation to embezzle or take bribes; weak values slowly rot, opening them to blackmail. | Missing | M | ★★★ |  | LogOld corrupt officials |
| NOB-24 | Secret vampire citizen | A vampire arrives as a migrant with a false birth date, never ages, feeds on sleepers; bodies pile up until it's caught. | Planned: Inbox | M | ★★★ | B04 | LogOld vampires, Bloat#249, Core96, PG60, PG97, WISHLIST Inbox |
| NOB-25 | Spies and double agents | Agents hide among migrants and visitors; dwarves get turned against the fort; some spies are found out. | Partial: goblin-spy stranger | L | ★★★ | B04 | Story:Spies and double agents, F16 hooded stranger |
| NOB-26 | Villain networks | A world villain recruits through agents using bribes, blackmail and flattery; a turned dwarf steals an artifact for later pickup. | Planned: B04 | L | ★★★ | B04 | LogOld villain networks, Log2020 traitors and heists, Arc High-level plots, 2016, 2020, PG55, PG162 |
| NOB-27 | Assassination plots | Someone arranges a noble's death by poison, ambush or "accident". | Planned: B04 | L | ★★★ | B04 | Story:Assassination plots, 2020 villainous plots |
| NOB-28 | Traitorous advisor | A trusted noble or post-holder works against the fort for gold or a grudge. | Planned: B04 | M | ★★★ | B04 | Story:Traitorous advisors |
| NOB-29 | Questers in the tavern | Strangers ask about the fort's artifacts; chatty dwarves let slip what's held; the quester steals or petitions for it. | Missing | M | ★★★ | B04 | LogOld questers |
| NOB-30 | Monarch arrives | Once the fort is grand and happy, the ruler moves in with entourage, lavish demands and treasury; satisfying them unlocks a Mountainhome goal and indirect control of armies and diplomacy. | Planned: Inbox | L | ★★★ | P02 | Core30, Req177, Log2020 monarch, LogOld monarch, WISHLIST Inbox |
| NOB-31 | Serving the crown | The monarch drafts soldiers from the fort for its wars; they return as veterans or not at all. | Missing | M | ★★★ | P02 | Core33 |
| NOB-32 | Succession in play | Dead post-holders and nobles are replaced messily: claimants, contested claims, usurpers. | Partial: elected mayor; worldgen succession | M | ★★★ | P02 | Core63, Req587, 2011 III.D, PG18, PG95, PG141, LogOld succession, Story:Claimants and proof |
| NOB-33 | Exile returns | A banished dwarf comes back with bandits or goblins and uses what they know of the gate. | Missing | M | ★★★ | B04 | Story:The exile returns |
| NOB-34 | Exiled lords | A deposed ruler from world history seeks refuge in the fort and brings their enemies. | Missing | M | ★★★ |  | Story:Exiled lords |
| NOB-35 | Pleas for help | A friendly civ begs the fort for aid; the overseer may send a squad. | Missing | M | ★★★ | P02 | PG10 |
| NOB-36 | Martial law | When order collapses nobles press guards into the army and impose martial law; everyone hates it. | Missing | M | ★★★ | P02 | Bloat#236, Req273 |
| NOB-37 | Public punishment | Executions and the stocks as spectacle; crowds attend and react by their values. | Missing | M | ★★★ | C07 | Story:Public punishment |
| NOB-38 | Hammerer and executions | An executioner post; serious crimes punished by hammering or death. | Planned: Inbox | S | ★★ | C07 | LogOld hammerer, WISHLIST Inbox |
| NOB-39 | Mandate flavour | Mandates explain themselves and name their penalty. | Partial: mandate message, blame | S | ★★ | C06 | Bloat#53, roadmap mandates |
| NOB-40 | Office envy | Nobles resent a rival getting a better office or bedroom. | Partial: room demands | S | ★★ | C06 | Log2020 office envy |
| NOB-41 | Advocate nobles | An animal advocate or philosopher lodges complaints about treatment and ethics with the mayor. | Missing | S | ★★ |  | Bloat#91, Bloat#133 |
| NOB-42 | Noble ransacking | A tantrumming noble sends guards to ransack homes. | Partial: tantrums topple furniture | S | ★★ |  | Bloat#82 |
| NOB-43 | Export bans | A noble forbids exporting a good; selling it anyway angers them. | Partial: mandates | S | ★★ | C06 | LogOld export bans |
| NOB-44 | Offerings to the ruler | Gifts to a noble or monarch earn favour. | Missing | S | ★★ | P02 | Req131, Req174 |
| NOB-45 | Elections and complaints | Unhappy dwarves line up to vent at the leader; a lost election wipes the old mayor's mandates. | Partial: elected mayor | S | ★★ | C06 | LogOld elections |
| NOB-46 | Petitioners | Dwarves queue before a noble with disputes; good or bad rulings change moods and grudges. | Missing | M | ★★ |  | Story:Petitioners |
| NOB-47 | Leaders away or captured | A commander on an expedition can't form squads; a captured leader forces an election. | Missing | S | ★★ |  | LogOld leaders away |
| NOB-48 | Titles must be earned | A new noble or champion isn't respected until they prove themselves. | Partial: titles, epithets | S | ★★ |  | Story:Titles that must be earned |
| NOB-49 | Symbols of office | An item is made a post's symbol, becomes a named artifact the holder carries; it can be lost or stolen. | Missing | S | ★★ |  | Log2020 symbols of office |
| NOB-50 | One appointed champion | The baron names one champion among veterans. | Missing | S | ★★ |  | LogOld champion |
| NOB-51 | Corrupt enforcement | The sheriff chases only what his noble cares about, pockets confiscations, beats too hard. | Partial: hot-tempered sheriffs beat harder | S | ★★ |  | Story:Corrupt enforcement |
| NOB-52 | Prison life | Chains and cages in cells; prisoners fed by others, receive visitors, may escape. | Partial: jail zone | S | ★★ | C07 | LogOld prison terms, Story:Jail life, WISHLIST Inbox jail chains |
| NOB-53 | Nobility drives trade | Higher rank brings bigger caravans and opens new diplomacy. | Partial: titles exist | S | ★★ |  | Log2020 nobility drives trade |
| NOB-54 | Trade that dies off | Caravans taper off if merchants lose money and nobody negotiates. | Missing | S | ★★ | B02 | Log2020 trade dies off |
| NOB-55 | Surrender that sticks | Offering unconditional surrender really ends a war, at a price. | Missing | S | ★★ |  | Log2020 surrender |
| NOB-56 | First contact | The fort may be two peoples' first meeting, or carry their history; the envoy acts on it. | Planned: B02 | M | ★★ | B02 | Bloat#73 |
| NOB-57 | Messenger missions | Appointed messengers go out to make peace, declare war, improve trade or seek alliance. | Planned: B02 | M | ★★ | B02 | Log2020 messenger missions, 2025 |
| NOB-58 | Liaison negotiations | The liaison agrees what next year's caravan brings and what the homeland wants; one or two export goods fetch a premium. | Planned: B02 | M | ★★ | B02 | LogOld diplomacy, Log2020 export agreements |
| NOB-59 | Patrons fund works | A wealthy noble pays for a monument in their name. | Missing | S | ★★ |  | Story:Patrons fund works |
| NOB-60 | Bounties | A price on a wanted figure or villain's head that heroes can claim. | Missing | M | ★★ |  | 2012-2020 Thief/Hero roles |
| NOB-61 | Counterintelligence screen | Rumours and reports drawn into an org chart up to the hidden master. | Planned: B04 | M | ★★ | B04 | Log2020 traitors, Story:Rumours screen |
| NOB-62 | Guild politics | Workshops belong to guilds that shape mandates and punishments. | Planned: B09 | M | ★ |  | Bloat#62, Bloat#305 |
| NOB-63 | Lethal-feud breaker | Lethal fights between residents are periodically called off and handed to justice. | Partial: duels to a yield | S | ★★ |  | Log2020 feud breaker |
| NOB-64 | Patrols after a crime | Guards watch a crime scene more closely for a while. | Missing | S | ★ |  | Story:Patrols after a crime |
| NOB-65 | Standing guard quotas | A justice noble mandates a number of guards and cages. | Missing | S | ★ |  | Req253 |
| NOB-66 | Guards and tax escort | Guard squad enforces law and escorts the tax collector; nobles complain if it's used elsewhere. | Missing | S | ★★ |  | LogOld guards |
| NOB-67 | Soldiers can't stand for office | Dwarves on duty can't be elected. | Missing | S | ★ |  | LogOld soldiers |
| NOB-68 | Consorts | A noble's spouse loses or keeps standing when the noble dies. | Missing | S | ★ |  | Req563, Req589 |
| NOB-69 | Spies who come home | Historical spies who settle drop false identities so the fort doesn't fight them. | Missing | S | ★ |  | Log2020 spies |
| NOB-70 | Noble dishes and decor | Nobles crave specific dishes and furnish to their temperament (dark or bright). | Partial: favourite foods | S | ★ |  | Bloat#85, Story:Decor that shows the owner |
| NOB-71 | Porters and rents | Rented rooms priced by demand; nobles bid for rooms and pay porters to carry their things. | Missing | M | ★ |  | Bloat#7-9, Bloat#83, 2012 R8 |
| NOB-72 | Settlements under the fort | Hamlets form nearby; noble rank follows attached sites. | Missing | L | ★★ |  | LogOld settlements |
| NOB-73 | Corrupting infiltrators | Magical beings hide inside society; inquisitions hunt them. | Missing | L | ★★ |  | 2020 myth and magic |
| NOB-74 | Law, rights and status | A framework of laws, customs and property so forts can be governed differently. | Missing | L | ★★ |  | 2016-2020 scenario framework, 2020 divine law |
| NOB-75 | Coins and wages | Minted coins, wages and bonuses, a treasurer tending the hoard (DF later dropped it). | Missing | L | ★ |  | Req100, Req102, Req394, Bloat#79, Bloat#96, Bloat#251 |

*Dropped or left to other domains: conquest/raids/five fates for a site (B03, military/world domain), baby-snatchers and werebeast curses (sieges/creatures), desecration curses (religion), threshold tuning in data files (engine), adventure-mode thief/disguise play.*

