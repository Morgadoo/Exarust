## ADDED Requirements

### Requirement: Debug new game
With `--new-game story[:N]` the first frame after the first load with a player SHALL start a story from background N (default 0) through the story start and the session begin, as the creator's BEGIN does: the first step SHALL be the new slot, and the story's level load and saves SHALL follow. The play level's player SHALL become the created character unless one was committed, and SHALL be despawned. It SHALL run once.

#### Scenario: A new story
- **WHEN** the load has ended and a player named Wanderer stands in the level
- **THEN** the flow is in story mode, its first step is the new slot, `exanima01` is loaded, the created character is Wanderer and the old player is gone

### Requirement: No debug career start
`--new-game arena` SHALL be refused with a message, because the arena career starts at the creator's commit, which the flow has no starter for.

#### Scenario: Asking for a career
- **WHEN** the launcher is given `--new-game arena`
- **THEN** it exits with code 2 and names `--open-creator arena`
