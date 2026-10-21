# Spec Delta

## ADDED Requirements

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
