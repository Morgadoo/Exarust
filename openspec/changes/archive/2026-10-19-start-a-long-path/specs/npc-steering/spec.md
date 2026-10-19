# Spec Delta

## ADDED Requirements

### Requirement: Start a long path
A path of fewer than two points SHALL NOT be followed, even when forced. Two points SHALL be followed when forced, or when the first and last are more than 150 cm apart. Exactly 150 cm SHALL NOT be followed.

#### Scenario: Exactly 150 cm stays, one centimetre more follows
- **WHEN** an empty path is forced, a single point is forced, two points 150 cm apart are not forced, two points 151 cm apart are not forced, and two points on top of each other are forced
- **THEN** the empty path and the single point are not followed, the 150 cm path is not followed, and the 151 cm path and the forced path are followed
