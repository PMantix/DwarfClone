# Body and mind: health, needs and moods

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## Health & body (HLT)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Wound model | Blows pass through armour, skin, fat, muscle, bone; materials decide cut/bruise/break; repeated blows to one spot add up | 7 parts with hp; wound grades Bruise/Gash/Mangle/Sever/Fatal; severed limbs, limping (`body.rs`, `game/combat.rs`) | No layers or organs, but grades are enough for drama; organs/nerves are the missing depth |
| Bleeding | Gradual loss; pressure, dressings and sutures slow it; wounded survive long enough to be helped | Quadratic loss: three cuts kill in ~100 ticks; ~60% of all deaths (`body.rs`) | The single biggest balance problem (S02 step 4) |
| Getting to care | Anyone carries the injured or unconscious to a bed, staunching on the way; patients wait for rescue | Nothing: the wounded lie where they fall (`game/medicine.rs`) | Core S02 work |
| Medical staff | Separate skills: diagnose, dress, suture, set bones, surgery; chief medic tracks patients; bad surgeons harm | One Medicine labor (suture, clean, splint, crutch); chief medic speeds by fit (`game/medicine.rs`, `game/positions.rs`) | No diagnosis/surgery; one skill |
| Hospital | A named location with beds, traction benches, tables, stocked cloth/thread/soap/splints | Hospital zone, beds used first, water carried to bedridden (`game/medicine.rs`) | No supplies stock, traction, staff assignment |
| Infection & disease | Dirty wounds infect; generic syndrome system for illness; (few true contagions) | Wound infection stops healing; miasma from rot sickens (`body.rs`, `game/miasma.rs`) | No contagion, fevers, organ effects |
| Poisons & syndromes | Venom, gas, contact and ingested poisons with paralysis, blisters, necrosis etc.; immunities | None | Whole system missing |
| Pain & consciousness | Pain by part size, fades with healing; nerve damage numbs or disables | Unconsciousness from wounds; no pain value (`body.rs`) | No pain, nerves, tolerance |
| Lasting damage | Scars, permanently weak parts, lost parts, old injuries carried from history | Lost limbs, crutches (`body.rs`) | No scars, prosthetics, historical injuries |
| Alcohol | One intoxication level: cheer, recklessness, stumbling, vomiting, passing out, death; dependence; keepers refuse drunks | Drink is a need; drinking contests at festivals (`game/needs.rs`, `game/festivals.rs`) | No drunkenness at all |
| Basic physiology | Hunger, thirst, sleep slow work; fat stores; heavy loads slow | Hunger, thirst, fatigue, drowning; focus scales speed (`dwarf.rs`, `game/needs.rs`) | Mostly there; no fat or load |
| Temperature & environment | Full heat model; frostbite, burning, cave adaptation (sun sickness), falls that catch on stairs | Blizzard chill, fire burns, magma kills (`game/weather.rs`, `game/fire.rs`) | No temperature, sun sickness, fall injuries |
| Hygiene & contaminants | Blood/mud/vomit spatter on tiles, creatures and gear; washing, soap, baths | Soap only for wound cleaning | No grime or washing |
| Appearance & aging | Hair/beards grow and grey, looks inherited, eyesight; aging | Age, sex, death of old age (`game/family.rs`) | No looks, decline, glasses |
| Attributes | ~19 physical/mental attributes that grow with use and rust | None; single fight_xp skill (W20 Later) | Planned W20 |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| HLT-01 | Press and bandage wounds | A bleeding dwarf stops and presses the wound, slowing the flow; a friend or anyone passing binds it with cloth or thread where they lie, and an untended dwarf eventually patches themselves. | Planned: S02 | M | ★★★ |  | Bloat#99; roadmap Bloat99; Log2009; Story:Battlefield Lunch |
| HLT-02 | Carry the wounded | The unconscious and crippled are carried to a hospital bed (or the floor if none is free) by a doctor or any hauler once the danger passes; carrier and patient both get thoughts. | Planned: S02 | M | ★★★ |  | Log2009; Log2020; roadmap Req11; Bloat#271; Story:cat |
| HLT-03 | Bleeding that leaves time to help | Reshape blood loss so a lost limb kills in minutes and a gash in hours; cumulative blows to one part still finish a long fight. | Planned: S02 | S | ★★ | S02 | Log2008 (tissue layers, cumulative wounds); inventory |
| HLT-04 | Drunkenness as a level | Every drink adds to one intoxication value: cheerful, then rash, stumbling, vomiting, passed out and (rarely) dead; tolerance varies and heavy drinkers come from personality. | Missing | S | ★★★ | C08 | Log2015; Bloat#373; roadmap Arc Crime/Bloat373/PG38/PG175; Story:Cold Fire; Story:Bride Price |
| HLT-05 | Sun sickness | Dwarves who have lived underground for a long stretch feel ill and retch on their first trips outside; it wears off with exposure. | Missing | S | ★★★ | D06 | Log2020; Story:Summoned to Darkness |
| HLT-06 | Contagious disease | Swamp fevers, rot or cursed objects start illnesses that spread between dwarves sharing dorms and dining halls; hardy dwarves resist, the hospital isolates. | Partial: wound infection, miasma | L | ★★★ | D05 | Bloat#144; roadmap Bloat144/Core74/2011 Wound II; Story:Olton at War |
| HLT-07 | Wait for rescue | A dwarf with a bad wound to a vital part stops working and lies still for help instead of limping on and bleeding out. | Missing | S | ★★ | S02 | Log2010 |
| HLT-08 | Scars, brands and lost teeth | Healed wounds leave named scars that last for life; others notice them and they can earn nicknames; brands come from punishment. | Missing | S | ★★ | D07 | Log2009; roadmap Req85/Core74/2011 Wound I; Bloat#333; Story:Monster Quest; Story:Dog-man |
| HLT-09 | Drink withdrawal | A dwarf without alcohol for days gets shaky, slower and irritable, then sick; meeting the need clears it. | Partial: drink need lowers focus | S | ★★ | C08 | Log2015; Story:Summoned to Darkness |
| HLT-10 | Drinking yourself to death | Without limits regulars can reach alcohol poisoning; tavern keepers refuse visibly drunk customers and level-headed dwarves stop on their own, both by personality. | Missing | S | ★★★ | C08 | Log2026 |
| HLT-11 | Retching at carnage | Gore, rot or a first slaughter makes squeamish dwarves vomit and lose a moment (leaving a mess to clean). | Missing | S | ★★ | D05 | Story:cat |
| HLT-12 | Surviving cave-ins | Falling rubble injures by size of the fall rather than always killing, leaving dwarves trapped and hurt. | Missing | S | ★★ | D06 | Bloat#110; roadmap Bloat110 |
| HLT-13 | Burning dwarves react | A dwarf on fire knows it, flees the burning room and rolls or jumps into water to put itself out. | Partial: fire burns dwarves | S | ★★ | D06 | roadmap Req480/Hero fire |
| HLT-14 | Old wounds from history | Migrants, veterans and beasts arrive already carrying injuries (a lost eye, a missing leg) and maybe a grudge about them. | Missing | S | ★★ | D07 | Log2011 |
| HLT-15 | Head injuries change people | Brain damage can blunt or flip a personality: the hothead turns placid or the gentle one cruel. | Missing | S | ★★ | D07 | roadmap PG158/PG141 |
| HLT-16 | Prosthetics | Crafted limbs beyond the crutch: peg legs, hooks, geared hands that a skilled mechanic improves. | Partial: crutch | M | ★★ | D07 | Bloat#35; roadmap Bloat35/PG45/2020; Story:Dwarven Servitude |
| HLT-17 | Old age decline | Eyesight, strength and speed fade with age, so old veterans miss more and old crafters slow. | Partial: aging, death of old age | S | ★★ | D07 | Story:The Beast |
| HLT-18 | Pain | Pain scales with the size of the hurt part and fades as wounds heal unless infected; tolerance varies, so some fight on and others collapse. | Missing | M | ★★ | D07 | Log2009; roadmap Req85/Core74; Story:A Terror to Behold; Story:Evil Philosopher |
| HLT-19 | Distinct medical skills | Diagnosis, dressing, suturing, bone setting and surgery are separate skills; a chief medic tracks each patient's needs; clumsy surgeons do harm. | Partial: one Medicine labor | M | ★★ |  | Log2009; Rel0.31 |
| HLT-20 | Hospital as a stocked place | The hospital holds its own cloth, thread, soap, splints and crutches and has assigned staff, possibly serving several zones. | Partial: hospital zone | S | ★ | S02 | Log2009; Log2020 |
| HLT-21 | Booze for the wounded | Caretakers bring patients ale and meals as well as water, and feeding a patient gives both sides a thought. | Partial: water only | S | ★ | S02 | roadmap Req573; Log2009 |
| HLT-22 | Cold injuries and frostbite | Shivering, burning lungs and lost fingers and toes for dwarves caught out in blizzards; extremities chill first; dwarves shelter in extreme weather. | Partial: blizzard chill; Planned: Inbox temperature model | M | ★★ | D06 | roadmap Req361; Log2008; Story:The Beast |
| HLT-23 | Falls that hurt | A long fall injures by height and may catch on stair levels or branches; falls become a real cause in the death report. | Missing | S | ★★ | D06 | Log2023 |
| HLT-24 | Tripping | Running over rough ground or mud, or being drunk, can trip a dwarf, sometimes into danger. | Missing | S | ★★ | C08 | Story:cat |
| HLT-25 | Vermin-borne plague | Rats and vermin in stores carry sickness, which makes cats, traps and clean food piles matter. | Missing | M | ★★★ | D05 | Bloat#18; roadmap Bloat18 |
| HLT-26 | Poisons and venom | Bites, blades and gases apply effects (paralysis, swelling, fever, necrosis) with visible spread; some creatures are immune; cures by antidote or cutting it out. | Missing | L | ★★ | D05 | Log2011; roadmap Req138/Req165/PG29; Story:The Evil Squire; Story:Dwarven Assault |
| HLT-27 | Contact poisoning by grooming | An animal that licks spilled booze or venom off its fur takes a dose; the drunk cat. | Missing | S | ★★★ | C08 | Log2011 |
| HLT-28 | Diagnosis, fractures and surgery | Patients are diagnosed once; simple breaks set, bad ones need traction, compound ones surgery; rotting tissue cut out; hopeless limbs amputated. | Planned: Inbox (diagnosis and surgery) | L | ★★ |  | Log2009; Log2020; roadmap 2020 |
| HLT-29 | Casts and dressings | Cloth dressings slow bleeding and speed healing, a cast lets a dwarf work one-handed, and sutures can tear open in a fight. | Partial: sutures, splints | M | ★ |  | Log2009 |
| HLT-30 | Hygiene and baths | Dwarves get filthy, wash with soap and buckets, have clothes cleaned, tidy their rooms, and enjoy bathtubs (gold ones for nobles). | Partial: soap for wounds | M | ★★ | D05 | Bloat#166; Bloat#170; roadmap Req512/Req431; Log2008 |
| HLT-31 | Blood and grime | Blood, mud and vomit pile up per tile, stick to feet, gear and fur, get tracked through halls, stain armour over wounds, and wash away in rain. | Missing | M | ★★ | D05 | Bloat#55; Bloat#319; Bloat#333; Log2009 |
| HLT-32 | Attributes that grow and rust | Strength, agility, toughness and mental attributes rise with use, fade with disuse, and feed combat, work speed and quality. | Planned: W20 | M | ★★ | W20 | Log2008; roadmap Core72 |
| HLT-33 | Needs slow work | Hunger, thirst and tiredness reduce speed and quality; dwarves finish the current job before eating unless desperate. | Partial: focus scales speed | S | ★ |  | Log2009; Log2022 |
| HLT-34 | Beards and hair | Hair and beards grow, grey and get styled; they can be singed, shaved as a prank or punishment, and losing one is shameful. | Missing | M | ★★ | D07 | Log2008; Story:Half Dwarf |
| HLT-35 | Strange afflictions | Parasites, wasting illness and transformations caught from evil lands or cursed places. | Missing | M | ★★ | D05 | roadmap PG116/PG167/PG183 |
| HLT-36 | Blind in the dark | Without light dwarves grope along walls, work slowly and fight badly. | Planned: B13 | M | ★★ | D06 | Story:Summoned to Darkness; Story:Orther Pushes His Luck |
| HLT-37 | Herbal remedies | An alchemist turns plant extracts into remedies and antivenom. | Missing | M | ★ | D05 | roadmap Req138/Bloat269 |
| HLT-38 | Exertion | Sprinting and long fights tire and sweat a dwarf, lowering speed until rested. | Missing | S | ★ |  | Bloat#232; Bloat#214; roadmap Bloat214/Bloat232 |
| HLT-39 | Heavy loads slow you | Carrying heavy things slows a dwarf, which gives wheelbarrows and carts a reason to exist. | Missing | S | ★ |  | Log2012 |
| HLT-40 | Choking | An unconscious dwarf can choke on vomit unless rolled over or tended. | Missing | S | ★ |  | roadmap Req198 |
| HLT-41 | Nerve damage | Motor nerve cuts disable a part, sensory ones numb it; movement needs a path from the brain. | Missing | M | ★ |  | Log2009 |
| HLT-42 | Fat and muscle | Well-fed dwarves grow fat (slower, but last longer starving); hard labour builds muscle; size matters in fights. | Missing | M | ★ |  | Log2009 |
| HLT-43 | Spectacles | Some dwarves see poorly; a jeweller grinds glass lenses and fits frames. | Missing | S | ★ |  | Bloat#33; roadmap Bloat33 |
| HLT-44 | Looks and heredity | Each dwarf has hair colour, build and features; children inherit them through simple dominant/recessive rules. | Missing | M | ★ |  | Bloat#176; Bloat#347; roadmap Core92/Bloat176/Bloat347; Log2009 |
| HLT-45 | Real pregnancy | Conception needs the couple to actually spend time together, with a chance of miscarriage. | Partial: marriage, children | S | ★ |  | Bloat#46 |
| HLT-46 | Children grow gradually | Children grow in size year by year and outgrow their clothes. | Partial: adult at twelve; Planned: Inbox (children outgrowing clothes) | S | ★ |  | roadmap Req441 |
| HLT-47 | Injury readout and medical history | Body parts coloured by state (useless, impaired, lost function) and a per-dwarf medical history tab. | Partial: wounds listed | S | ★★ | S02 | Log2009; Log2020 |

*Dropped as not fortress-relevant or owned by another domain: mending-speed bug fix (Bloat#127, Have), infections that kill weeks later and patients fed water (Have), poisoned water supply (Sieges), coal gas (Crafts), barbed weapons and mercy kills (Military), adventure-mode medical UI.*

## Mind, needs & moods (MND)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Personality | ~50 facets plus cultural values and beliefs, some hidden until shown | 5 facets -50..50 (anxiety, sociability, temper, diligence, artistry) (`dwarf.rs`) | Few axes; no values |
| Preferences | Likes for materials, creatures, plants, dishes, drinks, colours, shapes | Favourite material and food (`dwarf.rs`) | Narrow |
| Needs | About 30 needs driven by values (pray, argue, wander, see family, admire art, eat well...) | 7 needs, strength 1-3, focus 70-120% (`game/needs.rs`) | Many needs missing (Inbox) |
| Emotions & thoughts | Each event yields one or more named emotions picked by personality, with details of who/what/where | Thought { text, stress, tick } (`dwarf.rs`, `game/psyche.rs`) | No emotion types or details |
| Stress model | Slow long-term stress under a visible short-term mood; warning signs before breakdown | Stress accumulation with floor and misery counter (`game/psyche.rs`) | No two-layer mood, no warnings |
| Memories | Short-term slots for the year's strongest feelings, relived; some become lifelong and can change personality | None | Missing entirely |
| Breakdowns | Tantrum, depression, oblivious, berserk, catatonia; can recover or shift | Tantrums, melancholy (can lift), stark raving mad (`game/psyche.rs`) | No berserk, catatonia, transitions |
| Strange moods | Fey, secretive, possessed, fell (murder), macabre (bones); limited by likes | Fey-style moods with demands, artifact or madness (`game/psyche.rs`) | No dark moods |
| Relationships | Impressions, compatibility, trust/loyalty/respect; long-lived ties; arguing | Friend/grudge integer from chats (`game/psyche.rs`, `game/drama.rs`) | One axis |
| Grief & death | Grief needs proof of death; horror at risen kin; dwarves harden with exposure | Grief weighted by kinship, heroes, pets (`game/family.rs`, `game/pets.rs`) | No hardening, no body-proof |
| Status & envy | Room value thoughts, envy of others' rooms, noble requirements | Room value, noble room demands (`game/psyche.rs`, `game/positions.rs`) | No peer envy |
| Leisure | Taverns, temples, libraries, performances, hobbies | Breaks to pray or carouse; festivals (`game/needs.rs`, `game/festivals.rs`) | No hobbies or individual pastimes |
| Life goals | Lifelong dreams; fulfilment gives a permanent lift | None | Missing |
| Readouts | Thoughts and memories tabs, smiley mood strip, sort units by stress | Dwarf screen thoughts, status summary | No at-a-glance fort mood |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| MND-01 | Two-speed feelings | A visible short-term mood rides on top of slow stress, so a bad day isn't a ruin; only long misery breaks someone, and warning signs show before it does. | Partial: stress, tantrums, melancholy | M | ★★★ | C01 | Log2014; Log2020; roadmap 2018 pre-magic |
| MND-02 | Memories that come back | The year's strongest experiences sit in a few slots and are relived now and then; after a year each fades or becomes lifelong; shown as a life story by season. | Missing | M | ★★★ | C01 | Log2018; Log2022; roadmap 2018 pre-magic |
| MND-03 | Personality drift | As an old memory softens a dwarf may change a trait or value for good (a laughed-off embarrassment makes them bolder), and the sheet says why and when. | Missing | M | ★★★ | C01 | Log2018; Log2022 |
| MND-04 | Hardened by tragedy | Dwarves who see enough death, or work outdoors long enough, stop reacting as strongly, for good and ill. | Missing | S | ★★★ | C01 | Log2018; Log2020 |
| MND-05 | Vengeance vow | After kin or a close friend is killed, a dwarf swears to slay the killer or its kind; revenge eases the pain, the vow shapes their choices until kept or broken. | Partial: grudges, duels | M | ★★★ | C02 | Bloat#275; roadmap Bloat275; Story:Root |
| MND-06 | Berserk and shifting breakdowns | Add the berserk rage, and let breakdowns ease or turn into each other (melancholy to madness, rage to despair) instead of being final. | Partial: melancholy can lift | S | ★★★ | S09 | Bloat#89; roadmap Bloat89 |
| MND-07 | Traumatic triggers | A survivor of a fire or a beast panics when it returns, and dwarves avoid the place a friend died. | Missing | M | ★★★ | C02 | Story:The Beast; Story:Forest Befouled |
| MND-08 | Drowning sorrows | A stressed dwarf goes on a drinking spree that eases stress but can end in a tantrum or a brawl. | Missing | S | ★★★ | C08 | Story:cat |
| MND-09 | Victory cheer | Breaking a siege gives the whole fort a lift; the squad that did it gets a feast or extra rations from the overseer. | Partial: broken-siege milestone, no thought | S | ★★★ | C04 | Log2025; Story:cat |
| MND-10 | Life dreams | Some dwarves hold a lifelong goal (master a craft, raise a family, make a masterwork, slay a beast); reaching it gives a lasting calm. | Missing | S | ★★★ | D10 | Log2014 |
| MND-11 | Grief wandering | The bereaved drift to places tied to the dead (their workshop, their grave) and linger there. | Partial: melancholy; pets wait | S | ★★★ | C02 | Story:Root |
| MND-12 | Last words | A dying dwarf's final thought goes to the announcements and the chronicle. | Missing | S | ★★★ | C02 | Story:cat |
| MND-13 | Goading and dares | Dwarves question each other's nerve until one does something rash: picks a duel, hunts a beast, crosses the bridge during a siege. | Missing | S | ★★★ | C04 | Story:Mother of Death |
| MND-14 | Darker moods | Fell and macabre moods: the dwarf needs bones, a corpse, or a victim and makes a grim artifact; a dark mood can claim any dwarf, not only a few kinds. | Missing | M | ★★★ | C12 | Bloat#56; Bloat#237; roadmap Bloat56/Bloat237 |
| MND-15 | Horror at undead kin | A risen corpse that was a relative causes horror, and the ghost may haunt at the same time. | Partial: own dead rise (necromancer) | S | ★★★ |  | Log2019 |
| MND-16 | Ostracism | A dwarf who keeps misbehaving gets shunned, which feeds stress and further trouble. | Missing | S | ★★★ | C03 | roadmap Bloat105 |
| MND-17 | Buried resentment | An old slight (bullying, a lost promotion) lies dormant for years and surfaces as betrayal at the worst moment. | Partial: grudges, rivals | M | ★★★ | C02 | Story:Dragon Rider; Story:Dragon Quest |
| MND-18 | Rebellious youth | Children defy parents: they sneak into the barracks to spar, befriend the wrong dwarf, wander off. | Missing | M | ★★★ | C04 | Story:Mother of Death; Story:Snatcher |
| MND-19 | Envy of rooms, gear and titles | Dwarves compare rooms, gear and posts and resent those with more; nobles only care about what their rank requires. | Partial: room value, romantic jealousy | S | ★★ | C04 | Log2018; Story:The Order of the Dolphin; Story:The Evil Squire |
| MND-20 | Fort mood at a glance | A row of faces showing the fort's spread of mood, unit list sorted by stress, and a thoughts/memories tab with causes. | Partial: dwarf screen, status | S | ★★★ | C01 | Log2020; Log2021 |
| MND-21 | Grief needs a body | Mourning starts when the death is confirmed or remains are found (even a tooth); strangers' remains upset far less. | Partial: grief on death | S | ★★ | C02 | Log2011 |
| MND-22 | Many emotions | Each event produces named emotions chosen by personality; a death seen can stir horror, rage and grief together. | Partial: thoughts and stress | M | ★★ | C01 | Log2014 |
| MND-23 | Richer thoughts | Thoughts carry who, what and where, so reports and the overseer can quote them. | Partial: text thoughts | S | ★ | C01 | Bloat#262 |
| MND-24 | More needs | Wander, see family (paused if kin are away), admire art, eat good food, be creative, argue, own trinkets, see animals. | Planned: Inbox (more needs) | M | ★★ | D10 | Log2014; Log2026; roadmap 2012 R8 |
| MND-25 | Values and more facets | Many more personality facets plus values from culture (craftsmanship, family, martial prowess) that colour thoughts and needs. | Partial: 5 facets | L | ★★ | D10 | Log2014; roadmap 2012 R8 |
| MND-26 | Impressions and compatibility | Each meeting shifts short- and long-term impressions; whether a chat makes friends or enemies depends on matching personality, skills and tastes. | Partial: friendships, grudges | M | ★★ | C04 | Bloat#380; roadmap Bloat380; Log2008 |
| MND-27 | Long-lived ties and social clumping | Friendships don't fade just from distance; idle dwarves seek company in meeting halls and taverns, with more chat types. | Partial: meeting areas | S | ★★ | C04 | Log2020; Log2021 |
| MND-28 | Arguments | Quarrelsome dwarves argue when they talk; two arguers enjoy it, others are annoyed. | Partial: insults between rivals | S | ★★ | C04 | Log2015 |
| MND-29 | Woken by violence | A violent death or loud noise nearby wakes sleepers and ruins rest; a noise overlay shows the source. | Missing | S | ★★ |  | Bloat#88; Bloat#303; roadmap Bloat88/Bloat303 |
| MND-30 | Nerve under pressure | Fear makes hands shake: a frightened marksdwarf misses, a scared mason fumbles. | Missing | S | ★★ |  | Story:Mother of Death |
| MND-31 | Catatonia | A rare breakdown freezes a dwarf, unresponsive until someone close coaxes them back. | Partial: melancholy, madness | S | ★★ | S09 | Log2007 |
| MND-32 | Tantrums with limits | Raging dwarves tire (faster when hurt), break only what's in reach, and wreck their own bed only in the blackest mood. | Partial: tantrums | S | ★ | C12 | roadmap Req51/Req372/Req149 |
| MND-33 | Dreams and omens | Sleepers dream; some dreams foretell an attack or point to a hidden vault. | Partial: Whispering Stone | M | ★★★ |  | Bloat#128; Bloat#228; roadmap Bloat128/Bloat228; Story:Cado's Magical Journey |
| MND-34 | Petty misbehaviour | Shoving, drunken disorder and small vandalism return, with arrests by the sheriff. | Partial: tantrum crimes (W16) | S | ★★ |  | roadmap 2018 pre-magic |
| MND-35 | Pride in masterworks | A dwarf whose masterwork is traded, melted or destroyed is upset, far more if it was their only one. | Missing | S | ★★ | D10 | Log2013 |
| MND-36 | Admiring displays | Items in display cases and pedestals give admiration thoughts like fine engravings do. | Missing | S | ★★ | D10 | Log2016 |
| MND-37 | Coming home | Returning expedition members react to home, family and what changed: reunions, grief for news, pride. | Missing | S | ★★ | C02 | Story:Brotherly Love; Story:Warriors of the Dead |
| MND-38 | Job feelings by values | How a finished job feels depends on values: a tree-hugging resident hates felling or butchering. | Missing | S | ★★ |  | Log2014 |
| MND-39 | Culture shock | Migrants and visitors judge the fort by their own culture (an elf aghast at stumps and hides). | Missing | S | ★★ |  | Story:Forest Befouled |
| MND-40 | Family-event thoughts | Thoughts for a birth, becoming a parent, a new sibling, or a lost pregnancy. | Partial: births | S | ★★ |  | Log2009 |
| MND-41 | Animal sympathies | A dwarf who loves a kind of animal is upset to butcher or fight one. | Missing | S | ★★ |  | Bloat#167; roadmap Bloat167 |
| MND-42 | Wider likes and dislikes | Preferences for creatures, plants, dishes and drinks; a capped happy thought for seeing a liked animal. | Partial: material and food | S | ★ | D10 | Bloat#85; Bloat#87; roadmap Req190-191 |
| MND-43 | Food that matters | Dwarves seek valuable meals and favourite dishes and complain fort-wide about eating the same thing daily; nobles can be absurdly fussy. | Partial: favourite foods | S | ★★ | D10 | Bloat#85; Log2019; Log2026 |
| MND-44 | Hip flasks | Dwarves carry a flask and keep it topped up, sipping on long jobs or sieges. | Missing | S | ★★ | D10 | Bloat#45 |
| MND-45 | Smoking circles | Pipes and smokeleaf; dwarves gather in circles to smoke and chat. | Missing | S | ★★ | D10 | Bloat#43 |
| MND-46 | Toys and play | Children want toys and play with them; adults fiddle with puzzle boxes. | Partial: children play | S | ★★ | D10 | Bloat#3 |
| MND-47 | Hobbies | Idle dwarves pursue private pastimes: whittling, knitting, collecting pebbles. | Missing | S | ★★ | D10 | Story:Passion in the Arena |
| MND-48 | Mannerisms | A few habits per dwarf ("laughs at own jokes"), shown on their sheet and woven into reports. | Missing | S | ★★ |  | Log2009 |
| MND-49 | Children form tastes | Likes and some traits develop as children play and explore; many stay hidden until revealed. | Missing | M | ★★ |  | Bloat#252; roadmap Bloat252 |
| MND-50 | Remorse | Acting against one's values (fleeing a friend, striking a child) brings guilt and an attempt to make amends. | Missing | M | ★★ |  | Story:cat |
| MND-51 | Old grievances | Old dwarves repeat the same complaints until listeners grow tired of them. | Missing | S | ★★ |  | Story:cat |
| MND-52 | Crowd moods | Gathered dwarves cheer, jeer or panic together, and feelings spread through the crowd. | Missing | M | ★★ | C04 | Story:Passion in the Arena; Story:The Mighty Savager |
| MND-53 | Altruism | Kind dwarves help drunks to bed, lift the fallen and comfort crying children. | Missing | S | ★★ | C04 | Story:cat |
| MND-54 | Overcrowded animals turn mean | Animals kept too long in a cramped pasture grow irritable and violent. | Missing | S | ★★ |  | Log2012 |
| MND-55 | Humming at work | Cheerful dwarves hum or sing while working, lifting those nearby. | Missing | S | ★ |  | Story:Half Dwarf |
| MND-56 | Temperament fits the trade | Dwarves in jobs that clash with their personality are unhappier or worse at them. | Partial: post fit | S | ★ |  | roadmap Req549 |
| MND-57 | Privacy | Dwarves resent traffic through their bedroom. | Missing | S | ★ |  | roadmap Req389 |
| MND-58 | Moods limited by taste | Strange moods demand only materials the dwarf likes, pick reachable workshops and only make item kinds their culture knows. | Partial: moods with demands | S | ★ |  | Log2011; Log2026 |
| MND-59 | Stress tuning | Minor thoughts don't stick, migrants arrive calm, defending friends isn't stressful, the most fragile can still be helped. | Partial | S | ★ | C01 | Log2020 |
| MND-60 | Self-image versus true nature | A dwarf may think themselves brave or kind when they aren't; their acts follow the real traits. | Missing | M | ★ |  | Story:cat |
| MND-61 | Weariness of long life | Very old dwarves may sink into listless ennui and withdraw. | Missing | S | ★ |  | Story:cat |
| MND-62 | Shame shows | Blushing, hiding, tugging the beard, recorded in thoughts. | Missing | S | ★ |  | Story:Monster Quest; Story:Dwarven Assault |
| MND-63 | Reading people | A social skill lets a dwarf (or the overseer's spokesman) tell another's mood. | Missing | M | ★ |  | Bloat#374 |

*Dropped as adventure-mode or owned elsewhere: conversation-as-a-game (adventure), fear/morale for soldiers and brawls-stay-brawls (Military), sparring caution (Military); Have rows omitted: personalities core (Bloat#48), leisure on breaks (Req450), values create needs with focus (W09), burial and memorials (W10), sane-mood basics.*

