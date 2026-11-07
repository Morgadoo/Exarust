## ADDED Requirements

### Requirement: Attack start ends a parry
A side attack started while the fighter parries SHALL clear its parrying flag and, unless its pick was aimed at the foe's parry, release the parry's hold: its countered motion, every held item's parry grip and both hands' arm bit. An overhead or stab started while parrying SHALL end the parry in full before it starts, choosing the parry's stance again (Defence when failing or dazed, else the combat idle) unless the condition holds its clip still, and the attack SHALL return to that stance.

#### Scenario: Unaimed side attack from a parry
- **WHEN** a parrying fighter starts a main-side attack whose clip was drawn at random
- **THEN** it is no longer parrying, its hands lose the arm bit and its parry grip is disabled

#### Scenario: Aimed side attack from a parry
- **WHEN** a parrying Ward shield fighter starts a side attack aimed at its foe's parry
- **THEN** it is no longer parrying and its hands and parry grip stay held

### Requirement: Leaving an attack releases its hold
When a fighter's attack clip is left for another motion (its stance, a reaction or a chained attack) and it is not parrying, its countered motion SHALL be cleared, and a parry hold its aimed start kept SHALL be released.

#### Scenario: Aimed attack ends
- **WHEN** a side attack aimed from a parry returns to its stance
- **THEN** the fighter counters no motion and its hands and parry grip are released
