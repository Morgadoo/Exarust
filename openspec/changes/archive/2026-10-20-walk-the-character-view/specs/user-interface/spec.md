## ADDED Requirements

### Requirement: Character view chain walk
The character view's chain walk SHALL stop at a node whose record is missing, or whose flags carry both 0x12000000 with a negative word at 0x468; any other node SHALL follow its next node at 0x128.

#### Scenario: Settled, missing and open nodes
- **WHEN** the first node has no matching bits and points at a node carrying both 0x12000000 with a negative word
- **THEN** the walk stops at the second node
- **WHEN** the second node carries the bits with a word of 0 and points at a node without a record
- **THEN** the walk stops at the third node
- **WHEN** the only node carries one of the bits and no next node
- **THEN** the walk returns nothing
