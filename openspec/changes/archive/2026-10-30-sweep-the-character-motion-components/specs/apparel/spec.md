# Spec Delta

## ADDED Requirements

### Requirement: Texture layer strength
A stored texture layer's strength at a wear step SHALL be 0 from its maximum step on when that is above 0, its opacity × 1000 at step 0, that times `1 − decay·0.005` in double rounded half to even at step 1, and otherwise that share in single raised to the step through the engine's fast logarithm and exponential, rounded half to even.

#### Scenario: Step 1
- **WHEN** a layer of opacity 200 and decay 40 is at step 1
- **THEN** its strength is 160000
