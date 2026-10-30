# Spec Delta

## MODIFIED Requirements

### Requirement: Entry sections
The system SHALL read an entry's target, condition and value (attitude in byte 0, tolerance in byte 1) and its sections in the stored order: the unknown block, the dialogue graph, the encounter script, two unknown blocks and the interaction condition, each present when its kind bit is set. The corpse script (kind bit 0x20) SHALL be read after the sections of bits 0x1 to 0x8 only, where the original's corpse accessor reads it.

#### Scenario: A dialogue and an interaction condition
- **WHEN** an entry of kind 0x1042 aimed at role 0x1E holds a graph and the text `result = 1;`
- **THEN** its attitude and tolerance are 100, it has a dialogue aimed at 0x1E and its interaction condition is `result = 1;`

#### Scenario: The corpse script beside an interaction condition
- **WHEN** an entry of kind 0x60 holds one length-prefixed text `result = 1;`
- **THEN** both its corpse script and its interaction condition are `result = 1;`

## ADDED Requirements

### Requirement: Sections beside a corpse script
The sections other than the corpse script SHALL be read as the original's generic section accessor reads them: its kind table holds no corpse bit, so the 0x10 block is the corpse block when both bits are set and the interaction condition is the block after it.

#### Scenario: Corpse, 0x10 and condition bits together
- **WHEN** an entry of kind 0x70 holds the texts `corpse`, `ten` and `result`
- **THEN** its corpse script and its 0x10 block are `corpse` and its interaction condition is `ten`
