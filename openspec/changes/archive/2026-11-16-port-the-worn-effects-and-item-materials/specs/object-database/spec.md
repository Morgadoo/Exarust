## ADDED Requirements

### Requirement: Migrations run the item factory
The migration of a level's local tier when the level loads, of a save's world tier when it is imported and of the archive pass's store SHALL hand every migrated factory weapon (its model named `@…`) to the item factory's stat pass, once the game's data is open.

#### Scenario: A level with factory weapons
- **WHEN** a shipped level's local tier is loaded with the game's factory pass installed
- **THEN** its factory weapons' records differ from the migration without the pass
