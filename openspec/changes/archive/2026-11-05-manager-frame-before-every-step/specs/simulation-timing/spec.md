## ADDED Requirements

### Requirement: The manager frame before every step
The rise, the restore and the golem alarm's zone check SHALL run before every simulation step, alike with and without the bridge.

#### Scenario: An undead opponent rises
- **WHEN** a debug duellist with the story's body rules dies in the arena, once in one world and once behind the bridge
- **THEN** it rises at the same game time in both, on a step boundary
