## ADDED Requirements

### Requirement: Two-hand group
Every tick of a living character the hands, their parents and the second hand's grandparent SHALL take or lose a shared group bit by a mode: in combat 1 off its feet or walking, else 0; out of combat 0 while attacking, else 1; unarmed (no hands bits in the held weapon's class) 2. Mode 0 sets it on all five, mode 1 on the hands and parents less the second hand, mode 2 clears it; a death clears it.

#### Scenario: Armed and out of combat
- **WHEN** an armed character stands out of combat, then attacks
- **THEN** the right hand and both forearms share the bit, then the left hand and humerus too

#### Scenario: Unarmed
- **WHEN** the character's hands hold no weapon
- **THEN** no part keeps the bit

### Requirement: Shock end
A shocked body SHALL stay shocked while the game clock is at most the force release's busy deadline and SHALL become normal on the first posture past it (unsigned compare).

#### Scenario: At and past the deadline
- **WHEN** the clock reaches the deadline, then passes it
- **THEN** the body stays shocked, then is normal

### Requirement: Stagger capture
A stagger SHALL take the whole physical capture every time, also into a get-up clip already playing; only a clip that changed SHALL be blended and jumped in.

#### Scenario: Same clip
- **WHEN** a body staggers into the get-up it already plays
- **THEN** the clip keeps its clock and takes the whole capture

### Requirement: Relaxed blend in a tangle
The relaxed arm blend SHALL set both arms to 0.5 while the break-tangle counter is above 0.

#### Scenario: Tangled
- **WHEN** a body with arms at 0.9 blends relaxed with the counter above 0
- **THEN** both arms are 0.5
