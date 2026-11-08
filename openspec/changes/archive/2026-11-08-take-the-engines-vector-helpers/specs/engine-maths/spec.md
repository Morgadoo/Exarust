## ADDED Requirements

### Requirement: Engine fast vector reciprocal
The engine's fast vector reciprocal SHALL be the processor's approximate reciprocal of each component, as one packed operation on (x, y, z, 0): within 1.5·2⁻¹² of the exact value, a zero giving infinity of its sign and a NaN staying NaN.

#### Scenario: Each lane on its own
- **WHEN** the fast reciprocal of (3, −0.125, 10000) is taken
- **THEN** each component is the processor's scalar approximate reciprocal of that component, within 1.5·2⁻¹² of the exact one
