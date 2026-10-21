## ADDED Requirements

### Requirement: Morph target ordinal
A numbered morph target SHALL sort by its name's second and third characters (the name cut at 15 characters or its NUL) read as an optional `-` and leading digits, 0 without any.

#### Scenario: Two names
- **WHEN** the names are `M05Chest` and `Hair`
- **THEN** their ordinals are 5 and 0

### Requirement: Morph control match
A control triangle of two-component points, read from a rotation on, SHALL match its reference when each corner's squared distance, widened to double, is not above 1e-8; a NaN distance SHALL match.

#### Scenario: A rotated triangle
- **WHEN** the triangle (0, 0), (1, 0), (0, 1) is read from corner 1
- **THEN** it matches (1, 0), (0, 1), (0, 0) and not the unrotated triangle

### Requirement: Morph nearest records
Each base point SHALL copy the record of its nearest candidate by squared distance, strictly below the best so far starting at 3.4e38, and of candidate 0 when none is nearer.

#### Scenario: A NaN point
- **WHEN** a base point is NaN
- **THEN** it takes candidate 0
