## ADDED Requirements

### Requirement: Health regeneration
Each fixed tick a fighter not dead SHALL regenerate yellow health as the original: its rate is 0 while it attacks (condition 2) or plays Cmbt - Idle, else it rises 0.0003 × 0.01/83 a tick up to 0.01/83, and yellow gains √red × rate. Every blow and weapon contact SHALL set the rate to 0. A heal pool SHALL move 1.25e-5 a tick to red; yellow SHALL stay within red and red within the maximum. Death SHALL set yellow to 0.

#### Scenario: The retail trace
- **WHEN** a fighter with yellow 0.085 and red 0.203 of 0.25 leaves Cmbt - Idle for the Idle clip
- **THEN** its yellow is 0.0864 after 5 s, 0.1209 after 25.3 s (2,100 ticks) and 0.2009 after 45.8 s, the retail trace's 0.086, 0.121 and 0.202 within 0.0012

#### Scenario: The ramp
- **WHEN** a fighter's rate starts from 0
- **THEN** it reaches 0.01/83 on the 3,334th tick (40.2 s), and at it a human with full red heals 0.005 a second

#### Scenario: Held
- **WHEN** a wounded fighter attacks, or plays Cmbt - Idle at any tone
- **THEN** its rate is 0 and its yellow does not rise; in the Defence stance it heals

#### Scenario: A blow
- **WHEN** a blow lands on a fighter healing at the full rate
- **THEN** its rate is 0 and the ramp starts again

#### Scenario: Collapse and death
- **WHEN** a killing blow leaves a fighter's yellow at 0
- **THEN** it regains 0.00056 over the 249 ticks of its collapse, and its yellow is 0 from its death on

### Requirement: Red regeneration
Each fixed tick, dead or alive, a fighter whose rule bits hold red regeneration (0x20) without the bit that blocks it (0x100) SHALL regain 1.25e-5 red health, up to its maximum.

#### Scenario: Ogre
- **WHEN** an ogre (rule bits 0x67) with red 0.1 of 1.5 ticks 83 and 1,000 times
- **THEN** its red is 0.10104, then 0.1125

#### Scenario: Golem suit and humans
- **WHEN** a golem suit without its core (0x170), with its core (0x70) and a human (0x12) tick
- **THEN** only the suit with its core regains red

#### Scenario: Shipped characters
- **WHEN** the installed story levels' living characters and the arena's actors spawn
- **THEN** Sir and the three harvesters regenerate red in the story levels, and only the demon in the arena
