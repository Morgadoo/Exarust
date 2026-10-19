# Spec Delta

## ADDED Requirements

### Requirement: Barrier ripple
A barrier ripple SHALL lose 0.05 of strength each step, and the shown value SHALL move toward that strength by at most 0.04. Below 0.07 on both it SHALL be gone. While it remains, its place SHALL ease 0.05 of the way toward the hit. The dome extent SHALL be 30 plus 1.25 times the ripple count.

#### Scenario: A held ripple and an empty dome
- **WHEN** a ripple at shown 1 and strength 1 is stepped toward a hit 10 ahead, ripples at 0.06 and at 0.07 are stepped, and the extent of no ripples and of four ripples is read
- **THEN** the held ripple eases to 0.5, the two faint ripples are gone, the empty dome is 30 and four ripples make 35
