## ADDED Requirements

### Requirement: Creation list rebuild
Rebuilding SHALL build a list from the index and fail when the new list's count is not positive. Otherwise it SHALL store the index, release an existing list, store the new one, set the child's index, caption a present widget with the byte `index + 0x41` and draw when allowed.

#### Scenario: Build, empty and suspended
- **WHEN** the index is 2, an old list exists, the new count is 3, the caption widget is present and drawing is allowed
- **THEN** the index is stored, the old list is replaced and the new one stored
- **AND** the child index is 2, the caption is C and the draw runs
- **WHEN** the new count is 0
- **THEN** the rebuild fails and nothing is stored or drawn
- **WHEN** drawing is suspended
- **THEN** the list is rebuilt and not drawn
