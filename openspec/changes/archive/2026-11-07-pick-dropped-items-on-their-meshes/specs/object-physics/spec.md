## ADDED Requirements

### Requirement: Dropped items are picked on their meshes
The hand's pick SHALL meet a prop body put into the scene after the level was built (one with no stand-in collider) at the nearest front face of its parts' node meshes, a part without triangles at its collision spheres, nearer hits winning over the level; a body held in a hand SHALL be left out.

#### Scenario: A dropped bascinet
- **WHEN** the player presses on a bascinet dropped from the pack
- **THEN** the pick names it Loose and the press lifts it

#### Scenario: A held weapon
- **WHEN** the cursor lies over a weapon in a fighter's hand
- **THEN** the pick does not name the weapon

### Requirement: Released item's contacts cleared
When the item cursor lets go of an item (every release but the lock), the item's kept world contacts SHALL be dropped.

#### Scenario: A release
- **WHEN** a lifted item is let go
- **THEN** none of its parts keeps a world contact from before the release
