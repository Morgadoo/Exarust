## ADDED Requirements

### Requirement: Unresolved item report
An icon whose record has no model, or whose model never loads, SHALL be reported as a level-5 message titled "Item Error" with the text "Unable to resolve item: " and the id in eight hex digits; its panel entry SHALL keep its empty slot.

#### Scenario: An id no record holds
- **WHEN** an icon is reserved for an id with no record and the icon pass runs
- **THEN** the pending icon is dropped and the overlay shows "Item Error: Unable to resolve item: 000FFFF1"
