## ADDED Requirements

### Requirement: Item-panel mouse intent
The system SHALL map a mouse-down to no action, a drag start with the cell-centre grab, use, open or card; a right-click SHALL pass to the base form, and a drag start SHALL require drag permission and a hit.

#### Scenario: Drag, use and card
- **WHEN** the 0x4000 bit is set with drag permission and a hit
- **THEN** the intent is a drag start with grab (80, 80) for a 160 cell
- **WHEN** the 0x8000 bit is set on a container item
- **THEN** the intent is open
- **WHEN** the 0x2000 bit is set with a hit
- **THEN** the intent is card
- **WHEN** the right button is pressed
- **THEN** the intent is no action

### Requirement: Item-panel drag end
The system SHALL map drag end to no change, cancel, snap back, transfer to the linked panel, or removal.

#### Scenario: Four endings
- **WHEN** no entry carries the drag flag
- **THEN** nothing happens
- **WHEN** the drag is cancelled
- **THEN** it is cancelled in place
- **WHEN** drag-out is refused
- **THEN** the entry snaps back
- **WHEN** the destination is the linked panel
- **THEN** the entry transfers
- **WHEN** the destination is elsewhere
- **THEN** the entry is removed
