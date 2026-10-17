## ADDED Requirements

### Requirement: Fall sounds
A body falling to its death SHALL scream once with a heavy groan at its chest, at gain 0.2–0.35 and pitch 0.95–1.05 times its voice volume and pitch (the pitch from the first of two draws); the player SHALL hear `pitdeath01` at gain 0.1, not placed in the world, when the pit sounds.

#### Scenario: Draws
- **WHEN** the scream's two draws are 0 and 1, then 1 and 0.5
- **THEN** its gain is 0.35 and pitch 0.95, then 0.275 and 1.05

#### Scenario: The pit
- **WHEN** the player is lost and more than a second passes
- **THEN** `pitdeath01` plays once
