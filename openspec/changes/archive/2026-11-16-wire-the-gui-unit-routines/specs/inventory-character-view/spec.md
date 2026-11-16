## MODIFIED Requirements

### Requirement: Arsenal keys and right release
The open arsenal SHALL take each key of the frame with its word (a key-down or its repeat: Escape closes, the bound inventory key flips the embedded inventory; a typed character does nothing; nothing goes on to its parent) and a right release over its form, which closes it.

#### Scenario: Right release over the arsenal
- **WHEN** the player releases the right button over the open arsenal
- **THEN** the arsenal closes and the player's controls come back

#### Scenario: A held inventory key
- **WHEN** the inventory key goes down and repeats on the open arsenal
- **THEN** the embedded inventory flips on the key-down and again on the repeat

