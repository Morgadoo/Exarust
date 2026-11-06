## ADDED Requirements

### Requirement: Repeatable arena matches
An arena match the player enters SHALL stand its first round behind the loading screen, and the load SHALL wait for those bodies to be dressed and built. A `--fixed-frames` run SHALL read a fixed performance counter for the arena's draws.

#### Scenario: Two runs of an arena duel
- **WHEN** `play arena1 --opponent --weapon ub05ssworda --fixed-frames` runs twice with the same scripted input
- **THEN** both bodies' traces agree from the first tick
