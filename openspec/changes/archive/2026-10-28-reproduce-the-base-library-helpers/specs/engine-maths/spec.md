## ADDED Requirements

### Requirement: Engine vector helpers
The 3D length SHALL be `sqrt((x² + z²) + y²)` and the projection onto a unit SHALL scale the unit by `(x·ux + z·uz) + y·uy`. Component minimum and maximum SHALL give the second operand when either is NaN. The heading SHALL be zero below a squared XZ length of 1e-8, π at a cosine of -1 or less, zero above 1, the fast arc cosine otherwise, negated for a negative x.

#### Scenario: Projection order
- **WHEN** (2^-24, 1, 2^-24) is projected onto (1, 1, 1)
- **THEN** each component is 1 + 2^-23

#### Scenario: NaN in a component minimum
- **WHEN** (NaN, 2, NaN) and (1, 5, 3) are combined by the component minimum
- **THEN** the result is (1, 2, 3)
