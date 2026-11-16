## MODIFIED Requirements

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

### Requirement: Wearing and wielding a take
Wearing SHALL need its object in the scene, run the equip conflict pass, then put the item on and out of the carried list before taking it from its source. Wielding SHALL mark the item's first unmarked carried entry held and leave the hands to the game's inventory, as stowing does.

#### Scenario: A second weapon is wielded
- **WHEN** a searcher already wielding a sword wields a dagger it took
- **THEN** the dagger's carried entry is marked held and the game's take into the hand holds it

#### Scenario: A wear without its object
- **WHEN** a piece is worn from a container no longer in the scene, then from one that is
- **THEN** the first wear changes nothing and the second puts it on

## ADDED Requirements

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
