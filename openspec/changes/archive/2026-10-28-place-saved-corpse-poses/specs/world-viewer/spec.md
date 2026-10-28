# Spec Delta

## MODIFIED Requirements

### Requirement: Debug corpses
In play mode, `exarust --corpse x,y,z` SHALL lay a dead human of a new record down at that point (metres); having no saved pose, it SHALL be built standing and fall limp, unlike a level's placed dead. The flag MAY be given more than once.

#### Scenario: A corpse walked into
- **WHEN** `exarust --corpse 0,0,3.4 --press W@150-420 play arena` runs
- **THEN** a limp body falls and counts down to sleep where it was laid, and the player's lower legs meet it and wake it
