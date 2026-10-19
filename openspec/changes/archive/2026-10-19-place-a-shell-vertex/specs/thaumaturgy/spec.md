# Spec Delta

## ADDED Requirements

### Requirement: Shell vertex
A shell vertex SHALL sit at its position plus 2 times the normalized sum of its bone. A zero sum SHALL add nothing. A hit of 0.1 SHALL store about 3.333. A hit of 0.3 SHALL store 6. An eighth hit SHALL be refused.

#### Scenario: A raised vertex and a capped hit
- **WHEN** a position at 1, 2, 3 is offset by a sum of 0, 3, 0 and by a zero sum, a hit of 0.1 is stored, a hit of 0.3 is stored while seven are held, and an eighth hit is offered
- **THEN** the offset vertex is raised on its height, the zero sum stays at 1, 2, 3, the small hit is about 3.333, the 0.3 hit is 6, and the eighth hit is refused
