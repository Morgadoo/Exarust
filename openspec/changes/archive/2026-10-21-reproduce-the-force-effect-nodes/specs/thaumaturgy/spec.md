## RENAMED Requirements

- FROM: `### Requirement: Barrier padding`
- TO: `### Requirement: Barrier cast volume`

## MODIFIED Requirements

### Requirement: Barrier ripple
A barrier ripple SHALL lose 0.05 of strength each step, the subtraction taken in double precision, and the shown value SHALL move toward that strength by at most 0.04. Below 0.07 on both it SHALL be gone. While it remains, its place SHALL ease 0.05 of the way toward the hit. The dome extent SHALL be 30 plus 1.25 times the ripple count.

#### Scenario: A held ripple and an empty dome
- **WHEN** a ripple at shown 1 and strength 1 is stepped toward a hit 10 ahead, ripples at 0.06 and at 0.07 are stepped, and the extent of no ripples and of four ripples is read
- **THEN** the held ripple eases to 0.5, the two faint ripples are gone, the empty dome is 30 and four ripples make 35

### Requirement: Barrier update fraction
The barrier update SHALL compute its fraction as (target frame − base frame) + extra and its position as a + (b − a)·t, with a the previous snapshot's point and b the current one.

#### Scenario: One tick
- **WHEN** the previous point is (0, 0, 0), the current (10, 0, 0), the frames 5 and 3, and the extra 0.1
- **THEN** the fraction is 2.1 and the position (21, 0, 0)

### Requirement: Barrier cast volume
The Barrier's cast sound SHALL play at volume 0.2.

#### Scenario: The constant
- **WHEN** the cast volume is read
- **THEN** it is 0.2
