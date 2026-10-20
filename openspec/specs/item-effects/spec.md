# item-effects Specification

## Purpose
Tracks the character's active item effects: each held or worn item's tick,
its charge, and the glow it drives.

## Requirements

### Requirement: Item-effect glow
The system SHALL compute the glow intensity as 0.5·(0.01·level)^0.7, after stepping levels above 0x40 down by four and using 0x3c otherwise.

#### Scenario: Floor and first step
- **WHEN** the level is 0x3c or 0x40
- **THEN** the intensity is 0.5·0.6^0.7
- **WHEN** the level is 0x44
- **THEN** the intensity is 0.5·0.64^0.7

### Requirement: Item-effect charge
The system SHALL accumulate charge as min(current + source·0.004, source).

#### Scenario: Clamped charge
- **WHEN** the current is 99.9 and the source is 100
- **THEN** the charge is 100
- **WHEN** the current is 0 and the source is 100
- **THEN** the charge is 0.4

### Requirement: Item-effect entries
The system SHALL give a worn part a glow weight of 1 only for an odd kind below 0xff, and SHALL attach a keyed entry once.

#### Scenario: Glow and attach
- **WHEN** the kind is 1 and the byte 0
- **THEN** the glow weight is 1
- **WHEN** an entry with an existing key is attached
- **THEN** the list is unchanged
