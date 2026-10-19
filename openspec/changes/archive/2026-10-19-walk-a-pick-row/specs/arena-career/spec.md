## ADDED Requirements

### Requirement: Row walk
When the named check is clear, the gate SHALL open on the first record whose kind is 0x10010a or 0x10010d and whose link is the world. A different kind or a different link SHALL not open. Two matching records SHALL open once. A set named check SHALL open without a matching record.

#### Scenario: Kind 0x10010a linked to the world opens once
- **WHEN** an actor lists another kind, then kind 0x10010a linked to the world, then that kind again
- **THEN** the gate opens once

#### Scenario: A different link does not open
- **WHEN** kind 0x10010d is linked elsewhere, and then that link becomes the world
- **THEN** the first run does not open and the second opens once

#### Scenario: A named check opens without a matching record
- **WHEN** the named check is set and the only record has another kind
- **THEN** the gate opens once
