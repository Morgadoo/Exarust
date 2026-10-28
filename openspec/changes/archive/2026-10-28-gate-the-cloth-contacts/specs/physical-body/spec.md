## ADDED Requirements

### Requirement: Cloth contacts are opt-in
The level's cloths SHALL gather body and level contacts only when the cloth contacts switch is on (`EXA_CLOTH_CONTACTS=1`), and a cloth's bound radius SHALL be its particles' half-extent in centimetres scaled by its node. A sleeping cloth SHALL skip the gathering.

#### Scenario: The default level run
- **WHEN** exanima01 loads with the switch off
- **THEN** its cloths step with no contacts and the level keeps its frame rate

#### Scenario: The switch on
- **WHEN** a free cloth falls onto a level floor with the switch on
- **THEN** it gathers the floor's triangles and rests on them
