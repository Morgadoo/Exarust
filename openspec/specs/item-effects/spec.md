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

### Requirement: Item charge tick
A held item's charge tick SHALL run while its record kind's low five bits are 2 and its counter byte is below 0xff: the body's accumulator gains the source times 0.004 in double precision; while the sum is at or under the source the counter byte advances and the player's HUD is shown for 1000 ms, and once it passes the source it is held at the source and nothing else changes.

#### Scenario: Under and over the source
- **WHEN** the player's accumulator is 0 with a source of 100
- **THEN** it becomes 0.4, the HUD is shown and the byte advances
- **WHEN** it is 99.9 with a source of 100
- **THEN** it becomes 100 and the byte does not advance

### Requirement: Hand-effect dispatch
An item in either hand SHALL attach the electric tick, the glow tick or the flame tick when its power record's effect word is 1, 2 or 3, and nothing for any other word; the glow SHALL also give the item the level callback.

#### Scenario: A masked word does not count
- **WHEN** the effect word is 0x22
- **THEN** nothing is attached

### Requirement: Garment steps
Only an odd item kind SHALL step its counter byte: by one, or ten with the fast bit, per counter above one, set to 0xff when the step would reach it. A worn item's light SHALL turn on while the byte is still rising and off once it is not; an even kind is never rising.

#### Scenario: An even kind
- **WHEN** a lit worn item of kind 0x12 is ticked with both counters above one
- **THEN** its byte is unchanged and its light turns off

### Requirement: Material fade
The material fade SHALL write only the first material whose parameter word matches: the square of the value, and the material marked visible.

#### Scenario: Two matching materials
- **WHEN** two materials carry the parameter word and the value is 0.5
- **THEN** the first fades to 0.25 and the second is unchanged
