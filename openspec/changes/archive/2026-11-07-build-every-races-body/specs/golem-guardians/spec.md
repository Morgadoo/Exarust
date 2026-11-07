# Spec Delta

## MODIFIED Requirements

### Requirement: Golem bodies
The game SHALL build a simulated body for a placed character of every race but the demon (race byte 11), from its race model's bones and the template body's links, with its race's stature and the character init's muscles; such a character SHALL get a mind and a duellist like a human, and only the human frame (race bytes 0 to 2) SHALL wear human apparel.

#### Scenario: The golem races are physical
- **WHEN** the race bytes 16 to 20 are tested
- **THEN** each is built physical, and none is dressed

#### Scenario: The ghouls and the other races are physical
- **WHEN** the ogre (3), Sir (4), the Fael (5), the ghouls (6, 7), the gobbler (8), the manipulator (9), the harvester (10) or the wraith (14) is tested
- **THEN** each is built physical

#### Scenario: Other races stay static
- **WHEN** the demon (byte 11) is tested
- **THEN** it is not built physical
