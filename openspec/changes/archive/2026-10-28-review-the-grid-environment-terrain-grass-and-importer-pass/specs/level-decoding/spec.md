## ADDED Requirements

### Requirement: Grid border fill
Every grid read SHALL write plane A's border words in the loader's order: `0x20010007` over the whole top row, `0x00120070` and `0x12007000` down the left and right columns, `0x01200700` over the whole bottom row, then the corners `0x20130077`, `0x32017007`, `0x01320770` and `0x13207700`; an empty grid SHALL be left alone.

#### Scenario: A 4 × 3 grid
- **WHEN** a 4 × 3 zero plane A is filled
- **THEN** its rows are `20130077 20010007 20010007 32017007`, `00120070 0 0 12007000` and `01320770 01200700 01200700 13207700`

#### Scenario: One row
- **WHEN** a 3 × 1 grid is filled
- **THEN** it holds `01320770 01200700 13207700`

### Requirement: Placement tree entries
A `0x3DE0EC00` tree entry SHALL apply to the first direct child whose name equals the entry's (an empty name matching only an unnamed child), its nested entries to that child's children, and an entry whose child is missing SHALL be skipped with its nested entries; each record's next sibling SHALL follow its `(descendants + 1) × 0x4c` stride.

#### Scenario: An empty name under a named child
- **WHEN** a tree moves `Arm` with a nested empty-name entry, and an empty-name entry at the top, over a root whose `Arm` has an unnamed child
- **THEN** `Arm` and its unnamed child move and the root does not

### Requirement: Sub-chunk registry mode
With a handler registered, the reader SHALL hand every node sub-chunk to the first handler of its tag in place of its own sub-chunk handling; root chunks SHALL never reach the registry, and an empty registry SHALL read as the plain reader.

#### Scenario: A sector's sub-chunk
- **WHEN** a sector node carries sub-chunk `0x12345678` and the file a root chunk of the same tag, and a handler is registered for it
- **THEN** the handler sees the sector's body once, the node records no unknown sub-chunk, and the plain reader records the tag on the node
