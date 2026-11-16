## ADDED Requirements

### Requirement: Reconstruction strip leaves a dressed copy
A scanned player's story death SHALL copy the player's record (left-behind mark 0x40, roles 1 and 0xC2, yellow and red pools 0) into the world character store under its first free id and give the copy to the dead body, which stays dressed. The player SHALL go on with the record stripped of hands, worn and carried items, wearing the underwear, at 0x7FFF.

#### Scenario: A scanned player dies armed and dressed
- **WHEN** the death's strip runs on a player holding a sword and wearing a tunic
- **THEN** the body's record is a world record holding the sword and the tunic, and the kept record holds no item but the underwear

### Requirement: Dead player's body saved into the level
After the respawn or a stripped player's exit clears the body's owned mark, the story save SHALL write the stripped record as the player and the body as a dead character of the level section under its copy's world id, with the copy in the world character store.

#### Scenario: The respawn's save after a death in exanima02
- **WHEN** the respawn's save runs after a strip in exanima02
- **THEN** the exanima02 section holds a dead placement under the copy's id, and a load of that section with the saved world store places the corpse with its gear

### Requirement: Story level changes read the save's section
A story level change SHALL import the destination from the current slot's save when the save holds a section named by its locale, and from the archive otherwise.

#### Scenario: A door back to a saved level
- **WHEN** the player goes back to exanima02 after a save that wrote its section
- **THEN** the load reads the save's exanima02 section

### Requirement: Session arrival words kept across loads
A door's load SHALL name its entrance and the level it leaves, a respawn's load the session-start word; both words are cleared after the load. A pending respawn's save load SHALL set the session-start word and keep it, and every other load SHALL read the word the session last left with no previous level.

#### Scenario: A plain load after a door
- **WHEN** a save is loaded after the session took a door
- **THEN** the load reads arrival word 0 and no previous level
