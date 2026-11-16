## ADDED Requirements

### Requirement: Armour refresh after an armour rise
A conversion that raises the armour skill of the player or of a companion SHALL run the armour pass on that body again: Deflection read from its record and the layers rebuilt over the protection of the outfit it wears.

#### Scenario: Deflection learnt by a rise
- **WHEN** the player's armour skill rises to the level whose chosen technique is Deflection
- **THEN** the fighter's layers become those Deflection gives over its worn items

### Requirement: Companion rise on the body
A companion's rise SHALL put its new technique word on its body.

#### Scenario: A companion rises
- **WHEN** a companion's trained skill reaches a new level
- **THEN** its body's technique word is the word its skills now name
