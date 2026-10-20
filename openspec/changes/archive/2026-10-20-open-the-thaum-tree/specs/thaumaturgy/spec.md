## ADDED Requirements

### Requirement: Tree close
Close SHALL clear the global qword, run the inner reset once, clear the global byte and run the base once.

#### Scenario: The close
- **WHEN** the tree closes
- **THEN** the global qword and byte are cleared and one reset and one base call run

### Requirement: Tree show
Show SHALL call the focus once with `(word & 7) | 0x800` when the focus count is positive, refresh once when the pane word carries 0x20, OR bit 2 into 0x21d, write the clock plus the offset to 0x224 and store 0x1bc globally.

#### Scenario: The show
- **WHEN** the focus count is 1, the global word is 0xF, the pane word carries 0x20, the clock is 1000, the offset 250 and 0x1bc is 7
- **THEN** the focus word is 0x807 and one focus and one refresh call run
- **AND** 0x21d gains bit 2, 0x224 is 1250 and the stored value is 7
- **WHEN** the focus count is 0
- **THEN** no focus call runs
