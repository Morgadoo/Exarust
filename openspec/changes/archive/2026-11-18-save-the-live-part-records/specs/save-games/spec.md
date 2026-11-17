## ADDED Requirements

### Requirement: Saved part records
A save SHALL write the part records of each placed object's and prop's owned bodies and cloth from the live scene: a body's node its live world frame, nested under its parent body's record, and a moved cloth its live particles in the level's frame. A node without an attachment SHALL be passed over with its subtree, and an emitter SHALL end its siblings' records.

#### Scenario: A door left open
- **WHEN** a door's leaf is pulled open in exanima01 and the game saves
- **THEN** the door's placed object holds a record for the leaf at its live frame, and a load of the section spawns the leaf open

### Requirement: Unmoved part records
A stored part record whose node stands within a millimetre of it SHALL be written back unchanged, and an owned node with no stored record that has not moved against its parent SHALL write none; an object whose nodes did not move SHALL keep its stored records.

#### Scenario: Nothing moved
- **WHEN** no owned node of an object moved since the load
- **THEN** the object's records are the stored ones

### Requirement: Mechanism words in a save
A save SHALL write a mechanism's live object word as its placement word: a door's or sliding leaf's stored word and a lever's word whole, a push surface's state as the word's low byte.

#### Scenario: An armed lever
- **WHEN** a lever's word holds an armed return of 3.7 s and the game saves
- **THEN** the placement word is that word and a load of the section gives the lever the same word
