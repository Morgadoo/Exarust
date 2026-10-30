# Spec Delta

## ADDED Requirements

### Requirement: Protection figure order
The protection figure SHALL add, for each of the twenty bones with a nonzero coverage word c and weight w of the same index, the single-precision resistance sum times w·(√(c·0.01)·0.005) computed in double and rounded to single.

#### Scenario: First bone
- **WHEN** only the first coverage word is 200 and the resistances are 1, 0, 0, 0
- **THEN** the figure is √2·0.005 rounded to single
