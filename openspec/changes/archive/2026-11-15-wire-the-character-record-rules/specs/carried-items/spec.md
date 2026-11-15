## ADDED Requirements

### Requirement: Inventory pack counts the worn bulk
When the inventory window binds a character, the pack's used capacity SHALL be the bulk of its carried entries plus the bulk of every item the character wears.

#### Scenario: A dressed character's pack
- **WHEN** the inventory opens on a character carrying an item of bulk 3 and wearing items of bulk 5 and 7
- **THEN** the pack lists one entry and its used capacity is 15

### Requirement: Hand item dropped outside the pack
When an item in a hand is dragged off the inventory figure and dropped on a panel other than the character's own pack, the last carried copy of the item marked held SHALL leave the carried list, and each hand pair SHALL keep no more references to the item than the carried copies left, the second hand of a pair clearing first.

#### Scenario: The other set holds the item twice
- **WHEN** the main hand's sword, carried twice and marked held both times, is dropped into a body's pack while the stowed set holds it in both hands
- **THEN** the second held copy leaves the list, the stowed set keeps the sword in its first hand only, and the body's pack lists it

### Requirement: Unique sold item leaves the outfits
When a unique item leaves the arena company, it SHALL also leave the manager's and every member's outfit: a hand holding it SHALL be emptied and every worn copy SHALL come off.

#### Scenario: A sold unique piece
- **WHEN** a unique item the manager holds and a member wears twice is sold at the shop
- **THEN** the manager's hand holding it is empty and the member wears no copy of it

### Requirement: Picked-up items at the record's default cell
An item a searching character takes into its pack SHALL be listed at the record's default cell, `0x28, 4` in sixteenths of a GUI unit.

#### Scenario: A looted item
- **WHEN** a search adds an item to a character's carried list
- **THEN** the new carried entry sits at `0x28, 4`

### Requirement: Placed characters take their saved red and yellow
A character placed by a level SHALL start with yellow and red health equal to its record's stored shares times its maximum, computed in double precision; the record's focus, heal pool and potion SHALL not be loaded by this step.

#### Scenario: A saved level's wounded guard
- **WHEN** a placed record stores yellow 26214 and red 47185 for a body of maximum 0.25
- **THEN** the body's yellow is 0.1 and its red about 0.18, and its heal pool is unchanged

### Requirement: Skill-ready alert from the player's record
The HUD's skill-ready alert SHALL show while the player's record holds fewer than fifteen skill levels and no skill holds a technique ahead of its level; a player body with no record SHALL show no alert.

#### Scenario: Levels spent
- **WHEN** the player's skills hold fifteen levels
- **THEN** the alert is off
