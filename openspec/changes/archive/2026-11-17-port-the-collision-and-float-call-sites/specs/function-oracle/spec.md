## ADDED Requirements

### Requirement: Particle-matrix leaf routines checked
The oracle SHALL check the widened rest points, the 4×4 inverse and both products of the rest basis with the particles (the 4×4 drawn form and the 4×3 form) against their ports.

#### Scenario: The rest basis
- **WHEN** the four routines are replayed over 5000 cases each
- **THEN** every case is identical
