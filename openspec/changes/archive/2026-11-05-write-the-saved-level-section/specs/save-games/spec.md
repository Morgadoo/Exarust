# Spec Delta

## MODIFIED Requirements

### Requirement: Live world sync
A story save, the checkpoint's save and a level change SHALL first promote the player's local items to the world tier with the link remap table on and then rewrite the local doors' links; the player's possessions SHALL take the new ids, and the travelling followers' carried items SHALL be promoted the same way. A level change that carries the player's kit through the kit's temporary database SHALL not promote. An arena save SHALL, with the id remap table on, promote the manager's record (loadouts included), then every stash id, every entry of the arsenal's five shelves of five boxes and each employee's record and list, then rewrite the scene's local entity ids through the table, and turn the table off. A story or arena load SHALL import the save's world tier into the live database; a new story SHALL clear it.

#### Scenario: A repaired item through a save
- **WHEN** a kit copy of a marked template in the pack has its dirt and wear lowered, the story saves and the save is loaded
- **THEN** the pack names the copy's world id and the live world record keeps the lowered dirt and wear

#### Scenario: The arena's stores
- **WHEN** the stash names local 1, an arsenal box local 2 and an employee's hand, pack and list locals 5, 4, 3 and 1, and the arena saves
- **THEN** they become world 1, world 2, world 3, world 4, world 5 and world 1

## ADDED Requirements

### Requirement: Saved level section
Every save SHALL write the current level's section under its locale name after the world databases, holding the live local tier and its library and the placed objects as the scene holds them: a spawned object at its live placement with its live id and condition, a spawned object that is gone left out with the objects inside it, an unspawned one as loaded, and an object dropped since added at the end.

#### Scenario: A taken and a dropped object
- **WHEN** a spawned object holding another is gone and an item was dropped into the world, and the game saves
- **THEN** the section's list leaves both out, closes up the later container numbers and ends with the dropped item

### Requirement: Level from the save's section
A story load SHALL read the level from the save's own section when the package holds one, and SHALL not fill that level's containers.

#### Scenario: A dropped repaired item
- **WHEN** `exanima01`'s own trousers are repaired in the pack, dropped into the world, the story saves and the save is loaded
- **THEN** the level is read from the save's section with one more placed object and the trousers' record is the repaired one

### Requirement: Practice kit carry
The next practice fight SHALL prepare the player's own items into the kit's temporary database before the arena's level goes and equip them into the new level's local tier after it loads.

#### Scenario: The carry around the load
- **WHEN** the next practice fight's steps are built
- **THEN** the preparation comes before the level load and the equip after the practice filter
