## ADDED Requirements

### Requirement: Damage to health
Damage (yellow Y, red R, vital or not) SHALL take health as the original, subtracting in double precision: the rate to 0; yellow −0.1Y (−0.05Y with a potion); for rule bit 0x8 yellow −0.2R and red kept, else red −0.2R and yellow scaled by red's share left (0 with none). Under 0.05 the low-health check SHALL run, unless vital damage leaves yellow under 0.01 (double): yellow 0, in the story red −0.1 × maximum, killed. Non-vital damage SHALL leave yellow 0 and the fighter alive.

#### Scenario: Order
- **WHEN** a fighter at full health 0.25 takes Y 1 and R 0.5
- **THEN** its red is 0.15 and its yellow 0.09: cut by 0.1, then scaled by 0.6

#### Scenario: No lasting wounds
- **WHEN** a feral human (rule bits 0xC) and a human take Y 0.5 and R 0.3 from full health
- **THEN** the feral one has yellow 0.14 and red 0.25, the human yellow 0.152 and red 0.19

#### Scenario: Potion
- **WHEN** a human whose state flags hold the potion takes the same damage
- **THEN** its yellow is 0.171 and its red 0.19

#### Scenario: The lines
- **WHEN** vital damage of nothing reaches a fighter at yellow 0.01 (single precision), at 0.05 and just under 0.05
- **THEN** the first is killed, the second only hurt, and the third runs the low-health check

#### Scenario: Not vital
- **WHEN** non-vital damage leaves a fighter's yellow under 0.01
- **THEN** its yellow is 0, it lives, and the low-health check runs

#### Scenario: Story
- **WHEN** a killing blow lands on a fighter with red 0.2 of 0.25 in the story, and in the arena
- **THEN** its red is 0.175, and stays 0.2

#### Scenario: Story levels
- **WHEN** every living character of the installed story levels takes Y 0.5 and R 0.3
- **THEN** 248 of the 451 keep their red health (the 97 feral humans, 128 skeletons and 23 ancients) and the other 203 lose 0.06 of it
