# Spec Delta

## ADDED Requirements

### Requirement: World-load palette and container walk
For an item-body record whose armour type byte is not zero, the world-load walk SHALL hand the palette generator the palette byte's signed variance (bits 2–6 over 31 in double, negated by bit 7) plus its wealth (bits 0–1) before the refresh. A record with neither an item body nor a weapon section SHALL add each child of its container section whose item has a random table as a slot of that table, with the container's item limit.

#### Scenario: Palette byte
- **WHEN** an armour record's palette byte is 0x96
- **THEN** the generator receives −5/31 + 2

#### Scenario: Container children
- **WHEN** a container with item limit 40 holds two children naming a table and one plain item
- **THEN** the table gets two slots of limit 40 and the plain item none
