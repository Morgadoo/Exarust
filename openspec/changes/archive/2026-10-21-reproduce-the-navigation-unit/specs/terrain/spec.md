# Spec Delta

## ADDED Requirements

### Requirement: Vegetation scatter offsets
The system SHALL place each new point of the grass scatter at the cosine of the drawn angle times the drawn radius across and its sine times the radius along, with the engine's sine and cosine (one round-to-nearest reduction by quarter turns, single-precision polynomials).

#### Scenario: A quarter turn
- **WHEN** the drawn angle is a quarter turn
- **THEN** the new point lies straight along, not across, from its centre
