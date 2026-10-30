# Spec Delta

## ADDED Requirements

### Requirement: Terrain rule walk
At a rule, an ordered input at or below the low bound SHALL send the whole weight to the below child, one at or above the high bound to the above child, and any other input (in range or NaN) SHALL split it, the above child taking w·t first and the below child (1 − t)·w.

#### Scenario: Outside the bounds
- **WHEN** a rule's input is below its low bound and its above child is a rule with a NaN input
- **THEN** the whole weight reaches the below child and no total turns NaN

### Requirement: Terrain layer remainder
A painted layer's weight SHALL be its byte times the single-precision 1/255. A running remainder SHALL start at 1 and lose each painted layer's weight in turn; each layer and then the remainder (sent to the first rule) SHALL be walked only when its weight widened to double is above the double 0.001.

#### Scenario: Remainder order
- **WHEN** painted bytes 12 and 25 are walked
- **THEN** the remainder is (1 − w₀) − w₁ in single precision, 0.85490197

### Requirement: Terrain used materials and covers
Each leaf SHALL add ROUND(w·255) to its material's byte, comparing the low 32 bits of the 64-bit sum as a signed integer with 255. The tile SHALL use, in palette order, material 0 and every material whose summed leaf weight is above 1/510; their merged channel bytes SHALL be the tile's covers.

#### Scenario: Half a byte
- **WHEN** a material's summed leaf weight is exactly 1/510
- **THEN** it is not used

#### Scenario: A NaN leaf
- **WHEN** a leaf's weight is NaN
- **THEN** its material's byte is unchanged
