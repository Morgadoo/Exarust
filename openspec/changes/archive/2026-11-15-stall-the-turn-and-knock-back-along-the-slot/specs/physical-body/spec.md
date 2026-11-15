## ADDED Requirements

### Requirement: A hard hit stalls the turn
A stab's or overhead's hard weapon-body hit SHALL set its striker's turn stall to round(5√impact); each character tick SHALL take one off it before the turn, and the body SHALL not turn (no step, its error 0) while it is 1 or more.

#### Scenario: A stall of 3
- **WHEN** a hit sets the stall to 3
- **THEN** the turn holds on the next two ticks and turns on the third

### Requirement: The arms read last tick's turn error
The arms' turn floor (0.4·|error|, at most 0.4) SHALL read the error the last turn found before its step, 0 after a held turn, as the posture block reads it ahead of this tick's turn.

#### Scenario: A quarter turn
- **WHEN** a standing body turns toward a heading a quarter turn away
- **THEN** its arms read 0 on that tick and π/2 on the next

### Requirement: The turn lowers the support adapter
A body that turns SHALL lower its support adapter by 10·|error| in single precision, floored at 0 (a NaN gives 0); a held turn SHALL leave it.

#### Scenario: A small turn
- **WHEN** a steady body with the adapter at 0.99 turns with an error of 0.02 rad
- **THEN** the adapter is 0.79
