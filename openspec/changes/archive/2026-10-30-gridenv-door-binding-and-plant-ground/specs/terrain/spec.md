# Spec Delta

## MODIFIED Requirements

### Requirement: Plant instance matrix
The plant baker SHALL place each vertex through the row-vector rows `(cos·s, 0, −sin·s)`, `(0, s, 0)`, `(sin·s, 0, cos·s)` and the ground point, with the engine's sine and cosine of the stored angle, summing each coordinate as `(x·r0 + y·r1) + (z·r2 + t)`. Every variant's own normals SHALL turn by the same rows without the translation and be scaled by `1/s`, since the loader marks every variant so.

#### Scenario: Angle zero and a quarter turn
- **WHEN** a small-plant vertex at +X is baked at scale 2 onto the ground point (10, 1, −3)
- **THEN** angle 0 places it at (12, 1, −3) with normal +X, and a quarter turn places it within 1e-5 of (10, 1, −5) with a normal within 1e-6 of −Z

#### Scenario: A large plant's normal
- **WHEN** a large-plant vertex with normal +X is baked with a quarter turn
- **THEN** its normal is within 1e-6 of −Z

## ADDED Requirements

### Requirement: Plant ground height
Each plant SHALL stand at the height the plant ground query returns for its cell's four vertices, in centimetres on the original's 20 cm cells; the query's diagonal test SHALL pick the plane even where it is not the triangle under the point.

#### Scenario: Raised corner
- **WHEN** a 20 cm cell has only its first corner raised 1 m
- **THEN** the point (0.6, 0.2) stands at 0.2 m and the point (0.3, 0.3) at 0 m
