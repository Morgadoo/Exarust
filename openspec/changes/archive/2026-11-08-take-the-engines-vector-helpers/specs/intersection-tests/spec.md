## ADDED Requirements

### Requirement: Oriented-box slab reciprocal
The oriented-box slab tests SHALL scale the slab distances by the engine's fast vector reciprocal of the travel, not by an exact division, and fold them with the engine's component minimum, maximum, largest and smallest.

#### Scenario: An entry at the end of the segment
- **WHEN** a segment of length 8 along x starts 10 from the centre of a box of half extent 2
- **THEN** it hits exactly when 8 times the fast reciprocal of 8 is below 1
