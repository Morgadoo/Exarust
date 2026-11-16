## MODIFIED Requirements

### Requirement: Unresolved item report
An icon whose record has no model, or whose model never loads, SHALL be reported as a level-5 message titled "Item Error" with the text "Unable to resolve item: " and the id in eight hex digits. An id the loaded catalogue holds no model for SHALL get no icon slot (handle 0), and a panel's list load SHALL leave such an entry out.

#### Scenario: An id no record holds
- **WHEN** an icon is reserved for an id with no record
- **THEN** the handle is 0, nothing waits for the icon pass, and the overlay shows "Item Error: Unable to resolve item: 000FFFF1"

#### Scenario: A loot pack
- **WHEN** a looted body carries a shipped item and an id no record holds
- **THEN** its pack lists only the shipped item
