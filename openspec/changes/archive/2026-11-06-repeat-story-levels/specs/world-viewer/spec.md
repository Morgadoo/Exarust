## ADDED Requirements

### Requirement: The load holds the game
While the loading screen shows, game time SHALL stand still, and a pause the load made SHALL be released when it ends while a pause the game set stays. Behind the loading screen a body SHALL be built only once it is dressed, and a `--fixed-frames` run SHALL wait for every character to be dressed and every body and limb tree to be built before its first tick.

#### Scenario: Two runs of a story level
- **WHEN** exanima01, exanima02 or exanima06 runs twice with the same scripted input
- **THEN** every body's trace agrees from the first tick
