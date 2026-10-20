## ADDED Requirements

### Requirement: Morph part walk
The walk SHALL count a node whose kind masked with `0xFFF` is `0x89` and whose `0xC00000` bits are `0x400000`, then visit every child recursively. The number of eligible nodes SHALL be returned.

#### Scenario: Eligible, ineligible and child
- **WHEN** a node has kind `0x89`, flags with `0xC00000` equal to `0x400000` and a child that is also eligible
- **THEN** the count is 2
- **WHEN** the node's kind is `0x01`
- **THEN** the node is not counted
- **WHEN** the flag band differs
- **THEN** the node is not counted
