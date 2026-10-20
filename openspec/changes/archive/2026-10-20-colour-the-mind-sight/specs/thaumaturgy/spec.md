## ADDED Requirements

### Requirement: Sight entry colours
An entry SHALL take colour0 {0.1, 0.4, 0.6}, colour1 {0.8, 0.6, 0} and colour2 lerped from {0.4, 0, 0} to {0.31, 0.78, 0} by its third value. The alphas SHALL be 0.1 + 0.6·(1 − first) and 0.1 + 0.6·(1 − second) in double precision, and the quad alpha the overlay alpha less a positive entry fade.

#### Scenario: Zero and full values
- **WHEN** the three values are zero
- **THEN** colour2 is {0.4, 0, 0}, both alphas are 0.7 and the quad alpha is the overlay's
- **WHEN** the values are one and the fade is 0.2
- **THEN** colour2 is {0.31, 0.78, 0}, both alphas are 0.1 and the quad alpha is 0.2 lower

### Requirement: Pale entry
A kind-1 entry SHALL use {0.3, 0.3, 0.3} and {0.5, 0.5, 0.6} with alphas 1 and 0.5.

#### Scenario: Pale palette
- **WHEN** a kind-1 entry is coloured
- **THEN** the first two colours are the pale constants and the alphas are 1 and 0.5
