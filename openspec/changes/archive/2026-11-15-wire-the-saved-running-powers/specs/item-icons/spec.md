## ADDED Requirements

### Requirement: Icon cache cleared on a load
Every scene load SHALL empty the item icon cache and its atlas after the game screens are closed; an icon reserved afterwards SHALL take the first slot.

#### Scenario: A level change with icons cached
- **WHEN** a level change is asked while one item icon is cached
- **THEN** the cache holds no entry and the next icon reserved takes slot 1
