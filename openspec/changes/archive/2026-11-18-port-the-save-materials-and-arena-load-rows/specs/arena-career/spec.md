## ADDED Requirements

### Requirement: Unknown tournament event at load
An arena load SHALL rebuild a week's event whose saved id no tournament template has from the live board's row at that index (a zeroed row without a board), under the saved id, with the same rank, weapon row and word writes as a known event.

#### Scenario: An unknown event id
- **WHEN** a save's third tournament event names an id no template has
- **THEN** the loaded board keeps its other events and the third row is the live row under that id

### Requirement: Empty rival field at load
An arena load whose saved tournament field starts with company 0 SHALL draw a new field of 16 of the 19 rivals from the career's roll before the events are read, for both the board and the rival field.

#### Scenario: An empty field
- **WHEN** a save's tournament field is all zeros
- **THEN** the loaded rival field is the draw from the career's roll
