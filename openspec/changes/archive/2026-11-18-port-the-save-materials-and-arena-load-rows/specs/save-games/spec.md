## ADDED Requirements

### Requirement: Shared library materials in a saved section
A save's level section SHALL leave out of its material list every material name the game's shared material libraries hold, and SHALL write no material record of such a name.

#### Scenario: A level material a library holds
- **WHEN** a mesh of the saved level uses a material whose name a shared library holds
- **THEN** the section's material list does not name it and its library holds no record of it

### Requirement: Stash compaction of a converted arena save
Loading an old arena save that is converted SHALL drop, after the player's record is repaired and before the lists are regenerated, every hub stash entry whose id has the top bit set, keeping the others in order.

#### Scenario: A marked stash entry
- **WHEN** a converted arena save's hub stash holds a marked entry and a plain one
- **THEN** the loaded stash holds the plain one and no marked entry
