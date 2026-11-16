## ADDED Requirements

### Requirement: Stubbed matrix routines record their arguments
A spec SHALL be able to stub a routine that builds or reads a matrix through the x87 unit, answering its result from a buffer and recording its arguments into an out buffer, so the caller's own arithmetic is checked bit for bit.

#### Scenario: The head look
- **WHEN** the head look runs with its Euler routines stubbed
- **THEN** the kept yaw and the angles it builds the matrix from replay identical over 5000 cases
