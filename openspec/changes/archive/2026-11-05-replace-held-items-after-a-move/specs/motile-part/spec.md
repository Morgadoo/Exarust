## ADDED Requirements

### Requirement: Held item re-place after a move
When the game moves a character to a new place (the player's spawn placement on the ground, a Long Door), every item that character holds SHALL be put back at rest at its place under the holding hand where that hand now is, and woken. An item held flipped since its grip was built SHALL be turned half a turn about its own Y axis.

#### Scenario: A metre away
- **WHEN** a body holding a weapon is moved 1 m along x and 0.5 m along z and the re-place runs
- **THEN** the weapon's root part has moved by the same offset and its particles are at rest

#### Scenario: Flipped since built
- **WHEN** an item that was built unflipped is re-placed while held flipped
- **THEN** its place is the kept place turned half a turn about its Y axis, at the same point

### Requirement: Second hand slack after a re-place
After the re-place, a weapon of an armed grip class (0x30 bits) whose second-hand link is enabled and is not its first link SHALL take the distance that link now spans, summed `(x² + z²) + y²` under the root, as both its slack and its held length.

#### Scenario: Two-hander moved
- **WHEN** a two-hander with an enabled second-hand link is re-placed after a move
- **THEN** that link's slack and held length equal the span from its item point to its hand point
