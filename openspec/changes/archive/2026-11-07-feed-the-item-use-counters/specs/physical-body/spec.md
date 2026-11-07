## ADDED Requirements

### Requirement: Item use counters
The body tick SHALL feed two use counters, each sum in double: the movement's by 0.0005 times the locomotion speed (cm a tick) on each tick of the walk or the run and by 0.0025 on each tick a step runs (0.005 for a long step), and the attack's by 0.005 on each tick of an attack clip on its feet. The character's item-effect run SHALL read them and take one off each that is above one; with no effects it SHALL clear both.

#### Scenario: A walking body
- **WHEN** a body walks one tick at 10 cm a tick
- **THEN** its movement counter rises by 0.005

#### Scenario: An attack
- **WHEN** a body plays two ticks of an attack clip
- **THEN** its attack counter rises by 0.01
