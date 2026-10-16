## MODIFIED Requirements

### Requirement: Race body parameters
The system SHALL decode, for every record of the installed `races.rdb`, the body parameters the original reads: the clip-rate base, support-pin factor, size, scale, mass divisor, balance bonus, stance factor, arm relaxation, limb damping ratio, and four gait rows (walk, jog, sprint, backpedal) of thrust, push scale, brake distance (the loader's 0.5, 2, 2.5, 0.5 m), clip rate and boost speed. A missing race SHALL get the original's fallback: every factor 1.0 and gaits with no push.

#### Scenario: Human
- **WHEN** the installed `races.rdb` is parsed
- **THEN** every record decodes, and Human has support-pin factor 1.25, balance bonus 1.0, stance factor 0.5, arm relaxation 1.0 and damping ratio 1.0, with gait thrusts 88.9, 350, 472.2 and 77.8 per second and push scale 550 in every row

#### Scenario: Skeleton
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Skeleton has stance factor 0.75 and arm relaxation 0.5

#### Scenario: Unknown race
- **WHEN** a character's race byte names no race in the database
- **THEN** its body parameters are the fallback, whose gait table cannot move it

#### Scenario: Clip-rate base
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ghoul and Ghoul H have a clip-rate base of 1.5 and every other race 1.0

#### Scenario: Large races
- **WHEN** the installed `races.rdb` is parsed
- **THEN** Ogre has size 1.5 and scale 1.5, the harvester 1.8 and 1.8, Fael 1.5 and 1.35, Golem Guard 1.4 and 1.2, and Sir, Fael and the demon a mass divisor of 1.1

#### Scenario: Brake distances
- **WHEN** any race's gait rows are read, the fallback's included
- **THEN** a walking body brakes 0.5 m from its goal, jogging 2 m, sprinting 2.5 m and backpedalling 0.5 m

## ADDED Requirements

### Requirement: Feral sprint gait
The sprint gait of a character SHALL be its race's sprint row when its feral level is 0, and otherwise every column of that row moved linearly toward the race's jog row by the feral level.

#### Scenario: Half feral human
- **WHEN** a human of feral level 0.5 sprints
- **THEN** its sprint thrust is 411.1 per second, its clip rate 15.05 frames per second and its brake distance 2.25 m

#### Scenario: Zombie
- **WHEN** a human of feral level 1.0 sprints
- **THEN** it uses the jog row: thrust 350 per second
