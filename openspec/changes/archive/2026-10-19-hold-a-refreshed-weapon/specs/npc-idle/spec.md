# Spec Delta

## ADDED Requirements

### Requirement: Hold and weigh
When the clip base reaches 1 with a held record, that record SHALL be held again. A ghoul's arm stiffness SHALL be 1 and its hands SHALL weigh twice. A Fael's unarmed hands SHALL weigh four times, reach 150 cm, and set arm stiffness to one half.

#### Scenario: A ghoul weighs twice and a Fael weighs four times
- **WHEN** a knocked-out body whose clip base is 0.995 is ticked at 8001 ms holding fists as a ghoul, and another is ticked holding fists as a Fael whose unarmed reach is 150 cm
- **THEN** the ghoul's held balance is 1, its arm stiffness is 1, its hands weigh twice, and its reach is 50 cm, and the Fael's hands weigh four times, its reach is 150 cm, and its arm stiffness is one half
