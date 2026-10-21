## ADDED Requirements

### Requirement: Creation preview frame
The creation preview SHALL sit 180 units in and 24 down in its parent, the parent's width less 360 wide and its height less 28 high, without clamping.

#### Scenario: A 1024 × 768 parent
- **WHEN** the parent is 1024 × 768
- **THEN** the preview is at (180, 24), 664 × 740

### Requirement: Creation layouts
Form mode 12 SHALL take the ten-element layout table and build no editor sub-forms; every other mode SHALL take the seven-element table and build both editors.

#### Scenario: Mode 3
- **WHEN** the form opens in mode 3
- **THEN** it has seven layout elements and the editors

### Requirement: Creation drops
A drop onto the creation preview SHALL ignore item 8, go to the mode's own list when the form has one, to the apparel list in game mode 8, to the hook in game mode 0x1010, and otherwise by the item's kinds in order: apparel 0x1000 and weapons 0x4000 to their lists with the placing step, 0x8000 to the weapon list without it.

#### Scenario: An item of kind 0x8000
- **WHEN** an item whose record has only kind 0x8000 is dropped outside game modes 8 and 0x1010
- **THEN** it goes to the weapon list and is not placed
