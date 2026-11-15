## MODIFIED Requirements

### Requirement: Collapse drops the held items from the records
When a character's collapse starts, every hand that does not hold a shield SHALL let go of its item in the character's hands, and the character's possessions record SHALL lose those items from its out set together with their held entries in the carried list; a shield SHALL stay.

#### Scenario: Armed collapse
- **WHEN** a character holding a weapon in the main hand and a shield on the off arm starts to collapse
- **THEN** its main hand is empty, the shield stays, and on the next tick its fists are set up again

#### Scenario: Record drop
- **WHEN** a character whose record holds a sword in the main hand starts to collapse and lets the sword go
- **THEN** the record's main hand is empty and the sword's held entry is gone from the carried list

## ADDED Requirements

### Requirement: Came-loose hook
When the grip stage frees a held weapon whose holder is still in the world, the holder SHALL be told the weapon came loose, and its possessions record SHALL then lose every non-shield out-set item with its held entry, once per weapon; a freed shield or torch SHALL tell nothing.

#### Scenario: A weapon let go
- **WHEN** a character still in the world lets go of its held sword
- **THEN** it is told the sword came loose and its record's main hand is empty

### Requirement: Freed body link iterations
A held item freed by the grip stage SHALL become the root of its own tree with a link-iteration count of 2 plus its parts less one.

#### Scenario: A one-part weapon freed
- **WHEN** a one-part held weapon whose count was 15 is freed
- **THEN** its count is 2
