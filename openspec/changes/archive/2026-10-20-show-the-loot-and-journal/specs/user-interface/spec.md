## ADDED Requirements

### Requirement: Character loot show
An already inactive loot screen SHALL return true without work. Otherwise it SHALL set bit 0x20000000 in the owner's word, place the child when present and visible, run the base method once and return its result.

#### Scenario: The active loot screen
- **WHEN** the screen is not inactive, has an owner and a visible child
- **THEN** the owner is marked, the child is placed, one base call runs and its result returns
- **WHEN** the screen is already inactive
- **THEN** the result is true and the base method does not run

### Requirement: Journal show
The journal SHALL copy its x and y to the two origin globals, assign the string at the page's table offset and run the base method once.

#### Scenario: A journal page
- **WHEN** the page is 3
- **THEN** the origin copies, the table offset is 3 × 0x18 + 0x10 and one base call runs
