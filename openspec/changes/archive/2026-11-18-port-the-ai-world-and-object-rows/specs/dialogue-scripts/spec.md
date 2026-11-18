## ADDED Requirements

### Requirement: The crystal drain is the story hook's
The crystal drain SHALL run only in the story per-frame hook: with another hook, a covering main menu, a transition, a game not playing or a dying player the game second SHALL pass without a drain, and the first frame the hook runs SHALL drain once.

#### Scenario: The menu, a dying player, then play
- **WHEN** a charged crystal's inventory source passes a game second in the main menu's flow, then one with a dying player, then one while the story plays
- **THEN** only the last second drains it
