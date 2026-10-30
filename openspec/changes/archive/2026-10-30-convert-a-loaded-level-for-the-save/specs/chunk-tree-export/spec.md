# Spec Delta

## ADDED Requirements

### Requirement: Export preparation hooks
The exporter SHALL offer each mesh part to the scene's hooks from the last part down before collecting the node's materials, unless option bit 3 is set, which SHALL skip only that pass; SHALL offer a node's attachment (node flag 0x10000) before opening the node's chunk; and SHALL offer every character not of class 1 before writing the characters section. A node that shares another's data and has no mesh of its own SHALL take its master's parts.

#### Scenario: Hook order
- **WHEN** a two-part mesh, an attached body node and a character are exported with recording hooks
- **THEN** the calls are part 1, part 0, the attachment, the character
- **WHEN** option bit 3 is set
- **THEN** only the attachment and character calls remain and the material list is still written

### Requirement: Character condition words
The condition words SHALL be the health share and the dazed share of the maximum as value / max × 65535, the dazed share only when it is above zero (zero otherwise and for NaN), the third value × 65535, and 0xFFFF, each converted with round-half-to-even, a value that cannot be represented storing zero.

#### Scenario: Shares
- **WHEN** health 0.5, dazed 0.75, maximum 1 and third value 0.25
- **THEN** the words are 32768, 49151, 16384 and 0xFFFF

### Requirement: Loaded level conversion
A level section the reader decoded SHALL convert back into the exporter's scene: every node with its saved bit set and its stored name bytes, the props and placed objects with their part records as owned children numbered through their owners' scene indices, the placed characters with their pose trees, the grid, every material record present whatever the list's count, the object and character databases and the environment. A legacy flat pose SHALL be an error.

#### Scenario: A shipped level
- **WHEN** an installed story level is converted, written and read back
- **THEN** it has the same nodes, props, placed objects, local records, character placements, material records, databases and environment

#### Scenario: A list naming shared materials
- **WHEN** a material list counts two names and holds no record
- **THEN** the scan keeps no material and is not an error
