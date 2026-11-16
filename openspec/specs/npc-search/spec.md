# npc-search Specification

## Purpose
How a companion searches the nearby rooms: gathering containers, loose items and corpses, walking to each, looking it over, taking what beats its gear or telling its leader, re-arming and leaving behind what it no longer needs.

## Requirements

### Requirement: Standing and picking
An alert look SHALL make the tick idle. Standing SHALL gather once the anchor is 100 cm from the last gathering point. Picking SHALL take the reachable candidate within 1100 cm of lowest `(1 − d²/1100²) · e²` below 1100², with an arrival radius of 0.333 times the half-extents' sum plus 250 cm and an 8000 ms look; none left SHALL stand, idle.

#### Scenario: A chest ahead
- **WHEN** a searcher's anchor moves from 50 cm to 100 cm and a chest 150 cm ahead with half-extents of 30 cm is gathered
- **THEN** it stands idle at 50 cm, gathers at 100 cm, and picks the chest with an arrival of 279.97 cm and a look ending 8000 ms later

### Requirement: Holding the object
Past picking, a held object that left the scene, is beyond 1100 cm of the anchor or is unreachable SHALL send the search back to picking without letting the object go.

#### Scenario: The anchor walks away
- **WHEN** the anchor moves 1300 cm from a chest being looked at
- **THEN** the search picks again, finds nothing, stands, and the chest stays reserved

### Requirement: Looking and walking
Looking with attention on the goal SHALL end when the sweep sees it or the time is up. Attention on another SHALL attend and restart the 8000 ms; on no one SHALL let the object go. The path SHALL be forced to 100 cm under the object, stop 50 cm, radius 100 cm, mode 7.

#### Scenario: Distraction and the path
- **WHEN** someone else takes the attention at 5000 ms, then no one does, and in another search the goal is seen
- **THEN** the look restarts until 13000 ms, the chest is let go, and the other path goes 100 cm under the chest with stop 50 cm, radius 100 cm, mode 7

### Requirement: Arriving
Arriving (vertical distance halved) SHALL loot a container, a loose item not searched, or a dead character's worn items, settle (the idle motion, no clip of its own), and wait 1000 ms plus a draw of 1000. Three short walks SHALL let the object go. Its own pack open SHALL pause the search, idle.

#### Scenario: Short walks
- **WHEN** a searcher arrives at a chest, and another ends three walks 550 cm short with its pack open once
- **THEN** the first settles until 2031 ms, and the second pauses while the pack is open, paths again twice and then lets the chest go

### Requirement: Looting
Looting SHALL weigh one item each 1000 ms plus a draw of 500 and step back after taking one. A dead character's worn items SHALL come first, then its pack, skipping entries flagged `0x100` and ending at an empty one. When the source is exhausted the object SHALL be remembered (tagged `0x20000000` after a pack), a container tidied, a loose item taken last, and the object let go.

#### Scenario: A corpse
- **WHEN** a corpse wearing a coat and carrying a flagged item and a weapon is looted
- **THEN** the coat is worn, the flagged item skipped, the weapon carried, and the corpse remembered by its object id tagged `0x20000000`

### Requirement: Gathering
Gathering SHALL flood the 600 cm room grid from the anchor's cell through the four neighbours inside the border, flagged connected and closer than √11 cells. A container-like object SHALL be a candidate when searchable, not fixed, not searched and with an item record; a dead character's object when its pack is not searched. A character of a flooded cell whose object is container-like SHALL be weighed as that object, before the dead-character test. Each candidate SHALL be reserved.

#### Scenario: The flood's reach
- **WHEN** chests lie at the anchor's cell, three cells and one row off, three cells and two rows off, in a cell not connected, and fixed, searched, unsearchable or record-less ones, with a dead and a living character one row off
- **THEN** the anchor's chest, the one three cells and one row off and the dead character are gathered and reserved

#### Scenario: A character with a container-like object
- **WHEN** a living and a dead character of the anchor's cell each have a chest with an item record as their object, and the dead one's chest is searched
- **THEN** the living one's chest is gathered and reserved and the dead one's is not, though its pack is not searched

### Requirement: Item worth
Apparel SHALL be worth `min((s + 1) + (m + 1), (m + 1)/(m' + 1) + (s + 1)/(s' + 1))/2` over the carried and worn pieces it overlaps, s the bone-weighted coverage sum and m its largest bone value `c · 0.01 · 0.005 · p`. A weapon SHALL be worth its value plus 1 over the best carried of its class when that best is above 1.

#### Scenario: Gains
- **WHEN** a coat of bone value 0.2 is weighed over nothing and over an equal piece, and a weapon of value 5 against a carried one of 3
- **THEN** the coat is worth 1.2 and 1, and the weapon 1.5

### Requirement: Weapon value
A weapon's value SHALL be −10 at tier 0 and otherwise `thrust/140 + tier · ((q/255)²/2 + 1/2) · (1 − damage/255 · 0.3)`, × 0.8 for the `0x10` hand bits.

#### Scenario: Tier 2
- **WHEN** a tier 2 weapon of full quality, no damage and thrust 140 is valued, with and without the `0x10` hand bits, and at tier 0
- **THEN** it is 3, 2.4 and −10

### Requirement: Taking or telling
With a leader in sight a special piece, or one the leader gains more from past `max(0.55 − bulk · 0.05, 0) + 0.9` (apparel) or `(1.1 − v/10)² + 0.8` (weapons), SHALL be commented on. Otherwise apparel worth more than 1 SHALL be worn when its room exceeds 15 % of its bulk and carried otherwise, and a weapon not of the `0x06000000` kinds worth more than 1.05 SHALL be carried and the hands re-armed.

#### Scenario: Wear, carry or tell
- **WHEN** a coat worth 1.2 is weighed with nothing worn, a large piece over a worn one whose model byte is above 50, and a piece worth 3 to an empty-handed leader and 1 to the searcher
- **THEN** the coat is worn, the large piece carried, and the leader hears about the third

### Requirement: Comment memory
A comment SHALL forget the names said when more than 10 s passed and remember a new name, three at most. The first tables SHALL be used while fewer than two names are remembered, the pair tables for a name with "pair of". The draw SHALL reroll the table's last two and be marked on the table and its partner.

#### Scenario: Two finds
- **WHEN** "Helmet" is found at second 5 and "AXE" at second 15 with the generator at 0
- **THEN** the first uses the first table's first template and the second the further table's first

### Requirement: Comment text
`[a]` SHALL become the name with "a " or "an " by its first letter, `[i]` the name alone. The name SHALL be lower-cased but for its last character, and the phrase SHALL start with a capital.

#### Scenario: An axE
- **WHEN** "AXE" fills "And [a]." and "Owl" fills "[a], should you want it."
- **THEN** they read "And an axE." and "An owl, should you want it."

### Requirement: Re-arming and tidying
Re-arming SHALL take the best weapon in the main hand and stow the next best, one of the same hand group counting 0.6. Tidying after a container SHALL leave in it the weapon of lowest value over its class's best when under 0.8, and each carried non-special piece of apparel another overlapping piece makes redundant, `m/m' + s/s'` under 2.

#### Scenario: Best in hand, the rest left
- **WHEN** a searcher carries weapons of value 2.4 and 4, and tidies weapons of value 4 and 3 of one class and a carried piece half as good as a worn one
- **THEN** the 4 is wielded and the 2.4 stowed, and the 3 and the half-good piece are left

### Requirement: Takes carried out
A take SHALL despawn a loose item and remove a container's first matching entry. A worn take of the shield a dead body still grips SHALL let go of it, clear the second hand slot and remove the last held carried entry of it; any other worn take SHALL remove the item from the worn list. A carried take SHALL remove only the last held carried entry of the item and clear the fourth hand slot when it holds the same item.

#### Scenario: Wearing over what it overlaps
- **WHEN** a piece is worn over the underwear and a piece it overlaps, beside one it does not, and another piece is worn over a piece with a model byte over 50
- **THEN** the overlapped piece moves to the carried list, the underwear is dropped, the other stays on; the second wear changes nothing and takes nothing from its source

#### Scenario: Wielding keeps the item listed
- **WHEN** a carried item listed twice is wielded twice
- **THEN** both entries are marked held, both stay listed, and the hands do not change

#### Scenario: Each source
- **WHEN** a container, a corpse wearing one item and gripping a shield, and a loose item are taken from
- **THEN** each take edits its source, and the shield lets go, leaves the second hand slot and its held carried entry but not the worn list

#### Scenario: A carried take clears the fourth hand
- **WHEN** an unmarked carried item that the fourth hand slot also holds is taken
- **THEN** the fourth hand slot is empty and the entry stays listed

### Requirement: Drops carried out
Dropping SHALL wait while the pack is locked, need a carried entry of the item and a container of an object database, and refuse an item whose size is unknown or over the container's largest item; otherwise the item SHALL go into the container and its carried entry out of the pack, a hand that holds it keeping it.

#### Scenario: Sizes against the largest item
- **WHEN** items of sizes 4 and 5, one of no size, one only in a hand and one into an untagged container are dropped into containers taking items up to 4, first with the pack locked
- **THEN** the locked drop waits, the size-4 item goes in, and the others stay out

### Requirement: Apparel records as the search reads them
For an item record with an apparel body, the search SHALL read 20 occupancy words at 0x10, 20 coverage words at 0x150, the four protection bytes at 0x1A0, the special byte at 0x214, and the model section's byte at 0x1C. The apparel body SHALL be read before any weapon section. Bytes past a short section SHALL read 0.

#### Scenario: A worn piece
- **WHEN** a record has an apparel body covering bone 10 at 75 with protection 10, 20, 30, 40, and a model byte of 51
- **THEN** the search weighs it as apparel with those values, and once worn it leaves no wear room for a piece that overlaps it

### Requirement: Weapon records as the search reads them
For an item record with no apparel body, the search SHALL read the weapon section's flags word, quality (0x0C), damage (0x0D), thrust (0x33), tier byte (0x42, of which only the low five bits count) and special word (0x44). Without either section, an operative or container part SHALL make the item worth gathering. A record with none of these SHALL not be gathered.

#### Scenario: A sword record
- **WHEN** a weapon section holds flags 0x11, quality 200, damage 5, thrust 70 and tier byte 0xE3
- **THEN** the search weighs it as a one-handed weapon of 3 points: (70/140 + 3·((200/255)²/2 + 1/2)·(1 − 0.3·5/255))·0.8

### Requirement: Following the player in the running game
A level brain whose roles choose the follow SHALL follow the player. Inside the personal stop it SHALL stand. Inside the band (400 cm; 280 cm while following a route; 1200 cm while its search is busy) it SHALL run its search, anchored on the player, once a body tick. Past the band it SHALL plan a route to the player through the navigation grid on its planning turn and walk it. Combat, a flight or the scene taking the body SHALL stop the walk.

#### Scenario: The player walks away
- **WHEN** the player is 6 m from a standing follower that is not busy
- **THEN** the follower plans a route to the player and walks it

### Requirement: Searching the dead in the running game
The search's candidates in the running game SHALL include the level's dead bodies. Each SHALL be listed in its 600 cm room cell, with its worn items, its carried list and its hands. A take SHALL remove the item from the body's possessions and add it to the searcher's.

#### Scenario: A follower takes a dead body's sword
- **WHEN** a follower stands 1.5 m from the player, with a dead body 3 m away that wears a sword better than the one the follower carries
- **THEN** the follower gathers the body, looks it over, walks to it, settles and takes the sword: the body's worn list no longer holds it and the follower carries it

### Requirement: Searching the level's objects in the running game
The search's candidates in the running game SHALL be the placed objects the sector's lists hold. Each room cell SHALL list the objects of its body grid's parts, then those of its static grid's meshes, each object once a cell, with its first node's box and its tagged database id. An object with neither a body nor a static collider SHALL not be gathered. One straight under the level SHALL be searchable; placement bit 0x8000 SHALL mark it fixed.

#### Scenario: A chest and a loose blade
- **WHEN** a follower stands 1.5 m from the player, with a placed chest holding a better sword 3 m away and a loose blade of another class 2 m away
- **THEN** the follower takes both: the chest's section no longer holds the sword, the loose blade's object is gone, and the better weapon is in its hand, still listed and marked held

#### Scenario: Placed by the lists
- **WHEN** the lists hold an object in cells 10 and 11 and another in cell 10
- **THEN** cell 10 lists the first then the second, and cell 11 the first

### Requirement: Re-dressing after the search's takes
A body whose worn list the search changed SHALL lose from its outfit the garments of the items that came off and gain those of the items that went on, which asks the dress for a refit. A dead body that lost a worn item SHALL first get the underwear rule. A body whose hands changed SHALL take its new hands and main-hand weapon model.

#### Scenario: A robbed body
- **WHEN** a dead human loses its tunic to a search and wears nothing that covers the groin
- **THEN** its outfit loses the tunic's garment and its worn list holds the underwear

### Requirement: Phrase tables
The comment templates SHALL be the original's: for one item "I found [a].", "There's [a] here.", "Found [a] here.", "Got [a] here.", "[a] here, if you want it." and the never-drawn "This [i] could be useful."; for a pair the same with "some [i]" ("Some [i] here, if you want them.", "These [i] could be useful."); for a further item "And [a].", "Found [a] too.", "Also found [a].", "There's [a] too.", and for a further pair the same with "some [i]".

#### Scenario: A first helmet
- **WHEN** the first comment names a helmet and draws the first template
- **THEN** it reads "I found a helmet."

### Requirement: Approach answer
While approaching, its own pack open on screen SHALL answer not busy; a walking attention that takes the tick SHALL leave the answer busy and the state unchanged.

#### Scenario: The walk takes the tick
- **WHEN** the approach's attention of 5.1 takes the tick
- **THEN** the tick answers busy and the search stays approaching with no try counted

### Requirement: Wearing and wielding a take
Wearing SHALL need its object in the scene, run the equip conflict pass, then put the item on and out of the carried list before taking it from its source. Wielding SHALL mark the item's first unmarked carried entry held and leave the hands to the game's inventory, as stowing does.

#### Scenario: A second weapon is wielded
- **WHEN** a searcher already wielding a sword wields a dagger it took
- **THEN** the dagger's carried entry is marked held and the game's take into the hand holds it

#### Scenario: A wear without its object
- **WHEN** a piece is worn from a container no longer in the scene, then from one that is
- **THEN** the first wear changes nothing and the second puts it on

### Requirement: Weighing and writing back level objects
A level object's record SHALL be weighed from the game's object database or, tagged local, the level's own. A container's contents SHALL be read from its sections each tick and a changed container SHALL be written back, keeping the entries that stayed; a loose item taken SHALL leave the world with its colliders. The containers open in the player's popups SHALL count as open.

#### Scenario: A trunk is looted
- **WHEN** a searcher takes a jacket from a level trunk
- **THEN** the trunk's sections are written back without the jacket and keep the other entries

### Requirement: Re-arming through the inventory
In the running game, the re-arm's stow SHALL go through the planned arms: a one-handed weapon SHALL keep a light stowed first beside it and a two-handed one SHALL clear the second stowed item. The re-arm's wield SHALL mark the carried entry held and, for an item with a weapon section, take it into the hand through the inventory's take-in.

#### Scenario: A torch stowed first
- **WHEN** a searcher with a torch stowed stows a one-handed sword, then a two-handed axe
- **THEN** the sword is stowed with the torch beside it, then the axe is stowed and the torch's slot is emptied

### Requirement: The put-on's handling sound
A piece the search puts on SHALL play its handling sound at the body: its sound index names the foley class's set, at volume 0.3 and pitch `0.8 + 0.2·draw`; an index of 0 SHALL play nothing.

#### Scenario: A jacket put on
- **WHEN** a searcher with no leader in sight wears a piece of sound index 2 from a chest
- **THEN** one handling sound of index 2 is asked for at the searcher

### Requirement: Comments barked to the leader
A comment SHALL put its phrase in the speaker's `ContextPhrase` and activate the Explorer role's `ContextPhrase` topic toward the Leader role (30) with mode 1 in the speaker's memory of each qualifying contact.

#### Scenario: The installed roles
- **WHEN** a follower holding the Explorer role and the Leader relation toward the player comments with the installed roles
- **THEN** its `ContextPhrase` is the phrase and the topic is activated in its memory of the player

### Requirement: The leader in sight
The leader the search weighs for SHALL be the first contact attended 50 or more (an unordered attention passes) that is a character, not dead, holding the Player role.

#### Scenario: A dead leader
- **WHEN** a follower finds a special piece with its only contact, the player at attention 60, alive, then dead
- **THEN** it comments to the living leader and takes the piece itself when the leader is dead

### Requirement: A gripped shield taken off the dead
A dead body's shield taken by the search SHALL let go of its grip and leave the world with its node, the body's second hand and held entry cleared.

#### Scenario: The shield on the arm
- **WHEN** a searcher takes the shield a dead body grips
- **THEN** the shield's node leaves the world and the body's second hand and held entry are empty

### Requirement: The walk, settle and look the search asks for
The path and the approach SHALL drop the walk's urgency to 0. The arrival's settle SHALL take the look, heading and head look from the body's heading with a new sway draw, and the settle and loot states SHALL turn the body toward its look when no attention takes the tick.

#### Scenario: The chest walk
- **WHEN** a follower walks to a chest and settles beside it
- **THEN** its walk has urgency 0 and its look is the body's heading yaw

### Requirement: A container add's place
A new entry the search adds to a container SHALL come with no flags at the panel's width and height times 40.

#### Scenario: A four by four panel
- **WHEN** an item is added to a container whose panel is 4 by 4
- **THEN** the new entry is at 160, 160

### Requirement: Gathering by the database's mask
An object SHALL be worth gathering only when the database's mask test over its record's section chain finds an apparel, weapon, operative or container part.

#### Scenario: A chest and a cup
- **WHEN** a level holds a record with only a model and one with a container part
- **THEN** only the second is listed as loot

### Requirement: Container sizes from the flagged lookup
An item's size for a container add SHALL be its model section's byte `0x1C` as the database's flagged section lookup gives it: none when the record's table flags lack the model, or for a world id.

#### Scenario: Flags without the model
- **WHEN** two level records hold the same model section and only one's table flags name it
- **THEN** only that one has a size, and a world id has none
