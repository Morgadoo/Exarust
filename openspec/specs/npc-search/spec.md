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
Arriving (vertical distance halved) SHALL loot a container, a loose item not searched, or a dead character's worn items, settle, and kneel 1000 ms plus a draw of 1000. Three short walks SHALL let the object go. Its own pack open SHALL pause the search, idle.

#### Scenario: Short walks
- **WHEN** a searcher arrives at a chest, and another ends three walks 550 cm short with its pack open once
- **THEN** the first kneels until 2031 ms, and the second pauses while the pack is open, paths again twice and then lets the chest go

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
