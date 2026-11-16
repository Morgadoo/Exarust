## MODIFIED Requirements

### Requirement: Player contact list
The contact list SHALL hold the characters of the player's nearby list within 12 m, up to twelve, with closeness 1 − d·0.00083333335 capped at 0.99 and facing weight 0.75·(0.5 − 0.5·facing·dir) + 0.25·(0.5 − 0.5·cursor·dir)², each scoring half of both; a new character SHALL replace the lowest it outscores.

#### Scenario: Ahead and beside
- **WHEN** a character stands 100 cm ahead of the body and the cursor, and another ahead of the body with the cursor turned aside
- **THEN** the first's closeness is 0.9166667 with no facing weight, and the second's facing weight is 0.0625

## ADDED Requirements

### Requirement: The player's nearby gather and room move
On a level with a grid, the player's behaviour tick SHALL gather its nearby list from the room lists as a brain does, before its contacts, and move to its room's list after them unless staggered or suspended.

#### Scenario: Listed and unlisted characters
- **WHEN** one character linked in the player's room and one unlinked stand within 12 m of a player entering that room
- **THEN** the first tick has no contacts and the second's hold only the linked character
