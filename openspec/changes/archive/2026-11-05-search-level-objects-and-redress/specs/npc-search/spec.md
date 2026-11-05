## MODIFIED Requirements

### Requirement: Arriving
Arriving (vertical distance halved) SHALL loot a container, a loose item not searched, or a dead character's worn items, settle (the idle motion, no clip of its own), and wait 1000 ms plus a draw of 1000. Three short walks SHALL let the object go. Its own pack open SHALL pause the search, idle.

#### Scenario: Short walks
- **WHEN** a searcher arrives at a chest, and another ends three walks 550 cm short with its pack open once
- **THEN** the first settles until 2031 ms, and the second pauses while the pack is open, paths again twice and then lets the chest go

### Requirement: Takes carried out
A take SHALL despawn a loose item, remove a container's first matching entry, release an item from the corpse's grip (the second hand slot and its carried entry) or else remove it from the worn list, or remove a carried entry and clear the fourth hand slot when it holds the same item. Wearing SHALL run the equip conflict pass, then put the item on and out of the carried list before taking it from its source. Wielding SHALL fill the main hand and mark the item's carried entry held, clearing the old main item's mark; stowing SHALL fill the first stowed slot; both SHALL leave the item in the carried list.

#### Scenario: Wearing over what it overlaps
- **WHEN** a piece is worn over the underwear and a piece it overlaps, beside one it does not, and another piece is worn over a piece with a model byte over 50
- **THEN** the overlapped piece moves to the carried list, the underwear is dropped, the other stays on; the second wear changes nothing and takes nothing from its source

#### Scenario: Wielding keeps the item listed
- **WHEN** a carried item is wielded, then another
- **THEN** the first is marked held while it is in the hand and loses the mark when the second takes its place

### Requirement: Searching the dead in the running game
The search's candidates in the running game SHALL include the level's dead bodies. Each SHALL be listed in its 600 cm room cell, with its worn items, its carried list and its hands. A take SHALL remove the item from the body's possessions and add it to the searcher's.

#### Scenario: A follower takes a dead body's sword
- **WHEN** a follower stands 1.5 m from the player, with a dead body 3 m away that wears a sword better than the one the follower carries
- **THEN** the follower gathers the body, looks it over, walks to it, settles and takes the sword: the body's worn list no longer holds it and the follower carries it

## ADDED Requirements

### Requirement: Searching the level's objects in the running game
The search's candidates in the running game SHALL include every placed object with a model, listed in the 600 cm room cell of its first node's box centre with that box's half-extents and its tagged database id; one straight under the level SHALL be searchable, and a placement word with bit 0x8000 SHALL mark it fixed. Its record SHALL be weighed from the game's object database or, tagged local, the level's own. A container's contents SHALL be read from its sections each tick and a changed container SHALL be written back, keeping the entries that stayed; a loose item taken SHALL leave the world with its colliders. The containers open in the player's popups SHALL count as open.

#### Scenario: A chest and a loose blade
- **WHEN** a follower stands 1.5 m from the player, with a placed chest holding a better sword 3 m away and a loose blade of another class 2 m away
- **THEN** the follower takes both: the chest's section no longer holds the sword, the loose blade's object is gone, and the better weapon is in its hand, still listed and marked held

### Requirement: Re-dressing after the search's takes
A body whose worn list the search changed SHALL lose from its outfit the garments of the items that came off and gain those of the items that went on, which asks the dress for a refit. A dead body that lost a worn item SHALL first get the underwear rule. A body whose hands changed SHALL take its new hands and main-hand weapon model.

#### Scenario: A robbed body
- **WHEN** a dead human loses its tunic to a search and wears nothing that covers the groin
- **THEN** its outfit loses the tunic's garment and its worn list holds the underwear
