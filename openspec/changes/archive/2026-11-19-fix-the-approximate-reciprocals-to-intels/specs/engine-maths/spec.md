## MODIFIED Requirements

### Requirement: Engine fast vector reciprocal
The engine's fast vector reciprocal SHALL be the fixed approximate reciprocal of each component, as the original's one packed operation on (x, y, z, 0): within 1.5·2⁻¹² of the exact value, a zero giving infinity of its sign and a NaN staying NaN.

#### Scenario: Each lane on its own
- **WHEN** the fast reciprocal of (3, −0.125, 10000) is taken
- **THEN** each component is the scalar approximate reciprocal of that component, within 1.5·2⁻¹² of the exact one

## ADDED Requirements

### Requirement: Fixed approximate reciprocals
The approximate reciprocal and reciprocal square root SHALL be one fixed software answer equal to an Intel processor's `RCPSS` and `RSQRTSS`, the same on every machine, by a table lookup.

#### Scenario: The reciprocal of three
- **WHEN** the approximate reciprocal of 3 is taken on any processor
- **THEN** its bits are 0x3EAAA000

#### Scenario: An Intel processor
- **WHEN** the approximation is compared with an Intel Coffee Lake's instruction
- **THEN** every 32-bit input gives the same bits

### Requirement: Approximate sites
The swept box test, the weight normalisations, the sweep set-up, the level hit's centre fallback, the light basis, the polyhedron subdivision and the terrain normal SHALL take the approximate reciprocals where the original's instructions do.

#### Scenario: The sum normalisation
- **WHEN** weights are normalised to their sum
- **THEN** each is multiplied by the approximate reciprocal of the sum
