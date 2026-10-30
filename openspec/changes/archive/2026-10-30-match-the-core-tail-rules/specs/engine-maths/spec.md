# Spec Delta

## ADDED Requirements

### Requirement: One engine maths implementation
Every crate that may depend on the physics crate SHALL take the fast logarithm, the fast exponential and the approximate reciprocal square root from its engine maths module rather than a copy of its own.

#### Scenario: The damage model's power
- **WHEN** the damage model raises 4 to 0.5
- **THEN** it uses the shared logarithm and exponential and gets about 2
