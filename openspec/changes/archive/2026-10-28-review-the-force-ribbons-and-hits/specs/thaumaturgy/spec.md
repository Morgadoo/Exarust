## MODIFIED Requirements

### Requirement: Force character knock
A struck character SHALL be knocked by its gathered impulse, normalised, capped at 30 and doubled, when the impulse's squared length (summed `z² + (y² + x²)`) is above 1, and its support-pin gain SHALL be set to `(1 − k)·0.08` computed in double precision, 0 below 0 and 0.008 above 0.08 (compared in double), a NaN kept. Its instability counter SHALL rise by one and its concern toward the caster by 1 at gain 0.01.

#### Scenario: A heavy impulse
- **WHEN** a character of mass 80 gathers an impulse of 100 along x
- **THEN** it is knocked 60 along x with a pin gain of 0.078

#### Scenario: The length's sum order
- **WHEN** a character of mass 80 gathers an impulse of (0.2, 0.2, 1.3)
- **THEN** the knock is twice the root of 1.7699999, not of 1.7699997

## ADDED Requirements

### Requirement: Blast release alert
At the release, each character the caster's AI knows SHALL be told of the caster by `0.25·(1 − (d/1800)⁴)·(aim·dir + 3)` at gain 3 when within 1800 cm, a height difference under 150 cm counting as level.

#### Scenario: A known character ahead
- **WHEN** a known character stands 900 cm straight along the aim on the caster's level
- **THEN** the amount is `0.25·(1 − 0.0625)·4`

### Requirement: Force kept weight
A chunk struck at a limb or rebounding from a static body SHALL keep 0.9 of its weight computed in double precision; a piercing blast's chunk on a non-character limb SHALL keep 0.5 in single precision.

#### Scenario: A weight of 0.1
- **WHEN** a chunk of weight 0.1 rebounds
- **THEN** its weight is 0.09

### Requirement: Force level contact
A thrown blast chunk meeting the level within `reach·2 + margin + 10` SHALL move half way to the contact and keep 0.9 of its weight; a piercing blast's chunk SHALL instead, within `reach·2 + margin`, sit on the contact with weight 0 and wear 0.015. A running torrent point meeting the level within `width·0.25 + margin + 10` SHALL move half way to the contact. Limb contacts SHALL use the face walk for every blast.

#### Scenario: A bolt at a wall
- **WHEN** a bolt's chunk sweeps into a wall
- **THEN** it stops on the contact with no weight and is worn

### Requirement: Blast ribbon quad
Each blast quad SHALL widen its point by `(b − p) × (0, 0, 1)` scaled to the half width, where `p` is the previous call's point (the strip's first call taking `2·p₀ − p₁`), and SHALL then take its own point as the next call's previous point.

#### Scenario: An inner quad
- **WHEN** quads are emitted at (0, 0, 0) toward (10, 0, 0) and then at (10, 0, 0) toward (6, 8, 0) with half width 5
- **THEN** the second quad's vertices are (6, 3, 0) and (14, −3, 0)
