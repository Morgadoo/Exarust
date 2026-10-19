## ADDED Requirements

### Requirement: Skin cross
At unit 5 and point 10, 20 the cross SHALL draw eight corners: (8, 15), (13, 15), (8, 25), (13, 25), (6, 18), (6, 23), (16, 18), (16, 23). It SHALL count one begin, eight vertices and one line end, and SHALL bind no colour.

#### Scenario: The cross draws four segments
- **WHEN** the cross is drawn at unit 5 and point 10, 20
- **THEN** the eight corners are those listed, in that order
- **AND** the counts are one begin, eight vertices and one end
