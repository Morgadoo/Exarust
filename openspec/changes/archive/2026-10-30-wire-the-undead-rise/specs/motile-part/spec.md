# Spec Delta

## ADDED Requirements

### Requirement: Live undead rise
Each frame, a non-player character in the scene that is dead, whose rule word masked by 0x110 is 0x10, and whose rise clock the game clock is strictly past SHALL rise when its red health widened to double does not compare under 0.01; with less red and without the race's keep-rising flag its may-rise rule SHALL clear. A rise SHALL revive it with muscles built over the race factors and stats its body had when it went limp.

#### Scenario: Due and healthy
- **WHEN** a dead body with red 0.6 and a rise clock at 10 000 ms is checked at 10 008 ms
- **THEN** it is down and knocked out with yellow 0.3, and its drive carries its own race factors and stats

#### Scenario: Not yet due
- **WHEN** the same body is checked at 9 996 ms
- **THEN** it stays dead

#### Scenario: Revived character
- **WHEN** a body rises
- **THEN** its voice volume is its body scale times the race's voice factor, empty hands are set up unarmed, an AI brain applies its roles again and its rise clock is dropped

#### Scenario: Too little red
- **WHEN** a due body has red 0.005 and no keep-rising flag
- **THEN** its may-rise rule clears and it stays dead

### Requirement: Undoing go-limp
Going limp SHALL keep the body's clip player and its character's controller; once the body's muscles are back, both SHALL be given back, the body SHALL face the flattened direction from its pelvis to its chest, and the controller SHALL stand under the pelvis.

#### Scenario: Revived body
- **WHEN** a revived body's chest lies toward +x of its pelvis
- **THEN** it faces +x and has its clips and controller again
