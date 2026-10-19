# Spec Delta

## ADDED Requirements

### Requirement: Unarmed parry
A helpless fighter, a break-tangle motion, or yellow under 0.01 SHALL parry nothing. An attacker SHALL be attacking, not cleaving, in front, and within its reach plus 150 cm. Its clock SHALL sit from 0.3 through 0.9 unless the combo flag is set. Size 1.3 or more SHALL be refused without a shield. An unarmed parry SHALL succeed at speed 1 and blend 20. Motions 5 to 7 SHALL take a left clip, and 8 to 10 a right clip, the down clip when the table height is at or below the chest difference.

#### Scenario: A mid swing is parried and an early one is not
- **WHEN** a mid swing at clock 0.5 is in front and within reach, then the same swing is at clock 0.2, then that early swing has the combo flag, then yellow is 0.009, size is 1.3, the blow is behind, and the distance is reach plus 151 cm, and a down swing is read with the chest difference above the table and then below it
- **THEN** the mid swing is left-down at blend 20 and speed 1, the early swing is refused until the combo flag, yellow 0.009, size 1.3, the blow behind, and the far blow are refused, size 1.3 with a shield is accepted, and the down swing is left-down then left

### Requirement: Held bite
A depth above 0.1 SHALL open a grab whose pull is 1 and whose strain is the stick span. The solver link passes SHALL pull the two parts together. A depth of 0.1 SHALL open nothing, and those parts SHALL stay apart.

#### Scenario: A tenth stays apart and a quarter closes
- **WHEN** two parts 8 cm apart are solved with a bite of 0.25, then the same parts are solved with a bite of 0.1
- **THEN** the quarter bite leaves them less than a quarter of the gap apart, and the tenth bite leaves the gap where it started
