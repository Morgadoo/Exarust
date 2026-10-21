# Spec Delta

## ADDED Requirements

### Requirement: Rigid body translate
A translate SHALL move each particle of a rigid body by the offset times its weight, and with its restore flag add the offset's length to the body's strain and restore the six edge lengths once the strain passes 0.1 cm; without the flag the strain SHALL be left alone.

#### Scenario: Weighted translate
- **WHEN** a body is translated by 0.4 mm with weights 1, ½, 0, 2 and no restore flag
- **THEN** its particles move by 0.4, 0.2, 0 and 0.8 mm and its strain is unchanged

### Requirement: Rigid body placed at a matrix
Placing a rigid body at a matrix SHALL put its particles, their previous points and their tick's start at the matrix times the rest points, with its contacts and strain left as they are.

#### Scenario: At rest
- **WHEN** a shoved body is placed at a matrix
- **THEN** its next tick only adds gravity

### Requirement: Rigid body settling
While its settling countdown runs, a rigid body SHALL take each tick's gravity with no velocity carried or recorded.

#### Scenario: Settling
- **WHEN** a body settles for three ticks
- **THEN** each tick drops it by one tick's gravity, and the next tick's velocity is the last tick's drop alone
