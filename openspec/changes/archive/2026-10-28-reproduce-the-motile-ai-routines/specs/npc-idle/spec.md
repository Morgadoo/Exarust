# Spec Delta

## ADDED Requirements

### Requirement: The level-leave reset
When the level is written out, an AI character that is not collapsed, dead or dormant SHALL return its current health to its lasting pool when that pool is over 0.01, and SHALL forget its contacts and target. A roaming AI that is not a statue SHALL also, over two rounds, take a wander endpoint whose path spans more than 150 cm, followed by a home-side endpoint over the same span, and SHALL leave combat. Entering the scene SHALL NOT apply this reset.

#### Scenario: A roam is put down elsewhere
- **WHEN** a roaming AI with lasting health 0.6 and current 0.2 is reset, and the wander plans span 173 cm
- **THEN** its current health is 0.6, its contacts are empty, it leaves combat and it ends at the second wander endpoint

#### Scenario: A collapsed body
- **WHEN** a collapsed AI is reset
- **THEN** nothing changes

### Requirement: The idle object search
The idle driver's object search SHALL run over every object of the room, and the last usable one SHALL become the goal.

#### Scenario: Two usable objects
- **WHEN** a room holds two usable objects and two unusable ones
- **THEN** the later usable object is the goal
