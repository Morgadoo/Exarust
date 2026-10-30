# Spec Delta

## ADDED Requirements

### Requirement: Player health stored by a story save
Before a story save writes the player's record, the system SHALL store the player body's health into it: yellow and red as value / maximum × 65535 and focus × 65535, in single precision rounded half to even (red 0 unless positive), the seed 0xFFFF, a heal-pool entry when the heal pool is positive, then a potion entry with the expiry while the potion flag is set.

#### Scenario: Wounded human
- **WHEN** a human (maximum 0.25) with yellow 0.1, red 0.18, focus 0.4 and heal pool 0.03 is saved
- **THEN** the record holds pools 26214, 47185 and 26214, seed 0xFFFF and one heal-pool entry of 0.03

#### Scenario: Drunk potion
- **WHEN** a human with yellow 0.0625, red 0.15, focus 0 and a potion until 5400 s is saved
- **THEN** the pools are 16384, 39321 and 0 and the only entry is the potion's, 5400

### Requirement: Player health loaded at spawn
A player spawned from a record SHALL load its health: red then yellow as maximum × (stored / 65535) in double precision, focus as stored / 65535, and the heal pool and the potion (flag and expiry) from the saved entries, every known entry taking the last entry's value.

#### Scenario: Wounded human loaded
- **WHEN** the wounded human's save is loaded
- **THEN** the player has yellow 0.1, red 0.17999923, focus 0.4 and heal pool 0.03, without the potion flag

#### Scenario: Potion loaded
- **WHEN** the potion save is loaded
- **THEN** the player has yellow 0.06250095, red 0.15, the potion flag and expiry 5400

#### Scenario: Heal pool and potion together
- **WHEN** the saved record holds a heal-pool entry and then a potion entry with expiry 5400
- **THEN** the loaded heal pool is the bits of 5400 read as a float and the potion expiry is 5400
