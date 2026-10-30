# Spec Delta

## MODIFIED Requirements

### Requirement: Body item transfer
A drag from a body's panel dropped on another pack SHALL move the item from the body's possessions into the destination's carried list at the drop's cell, removing a worn item, a hand item with its held carried copy, or the last carried copy; a drop on the same panel SHALL only reposition the entry, and a drop outside every panel, or on an equipment panel other than as a body's accepted core, SHALL be refused.

#### Scenario: Taking off a worn item
- **WHEN** the worn item 0xa0 is moved to the player's pack at cell (40, 60) with unit 4
- **THEN** the body wears only 0xa1 and the player carries 0xa0 saved at (160, 240)

## ADDED Requirements

### Requirement: Body core drop
A drop on a body's equipment panel SHALL be taken when the body's core test accepts the item: the AL crystal for a race flagged 0x2000, crystal A for one flagged 0x1000, and neither while any of the five crystals is worn. The accepted crystal SHALL be worn; the AL crystal SHALL also carry the activation (mark and field cleared, the "Activate" dialogue, the activation byte).

#### Scenario: A core for a golem
- **WHEN** crystal A is dropped on the equipment panel of a body whose core test accepts it
- **THEN** the drop is taken onto that panel

#### Scenario: A refused core
- **WHEN** the AL crystal is dropped on the equipment panel of a body whose core test refuses it
- **THEN** the drop is refused and the item stays in its pack
