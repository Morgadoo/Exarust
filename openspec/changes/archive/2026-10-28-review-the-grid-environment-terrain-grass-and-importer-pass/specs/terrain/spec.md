## ADDED Requirements

### Requirement: Plant instance matrix
The plant baker SHALL place each vertex through the row-vector rows `(cos·s, 0, −sin·s)`, `(0, s, 0)`, `(sin·s, 0, cos·s)` and the ground point, with the engine's sine and cosine of the stored angle, summing each coordinate as `(x·r0 + y·r1) + (z·r2 + t)`. Grass and small-plant normals SHALL turn by the same rows without the translation and be scaled by `1/s`; every vertex of a large plant SHALL take the terrain normal at its point.

#### Scenario: Angle zero and a quarter turn
- **WHEN** a small-plant vertex at +X is baked at scale 2 onto the ground point (10, 1, −3)
- **THEN** angle 0 places it at (12, 1, −3) with normal +X, and a quarter turn places it within 1e-5 of (10, 1, −5) with a normal within 1e-6 of −Z

#### Scenario: A large plant's normal
- **WHEN** a large-plant vertex is baked where the terrain normal is (0, 0.6, 0.8)
- **THEN** its normal is (0, 0.6, 0.8)

### Requirement: Terrain normal under a plant
The terrain normal at a point in vertex steps SHALL blend the four vertex normals of its truncated cell corner `(i, j)` with weights `fx·fy`, `(1−fx)·fy` for `(i+1, j)`, `(1−fy)·fx` for `(i, j+1)` and `(1−fy)(1−fx)` for `(i+1, j+1)`, in that order, and normalise by the approximate reciprocal square root (a zero sum stays zero); a point past the grid SHALL give +Y.

#### Scenario: Blended corners
- **WHEN** a 3 × 3 grid holds +X at (0, 0), +Z at (1, 0) and +Y elsewhere and is sampled at (0.75, 0.5)
- **THEN** the sum is (0.375, 0.5, 0.125) normalised by its reciprocal square root

### Requirement: Vegetation density threshold in double
Nothing SHALL grow at a point whose density, widened to double, is below the double 0.35; a NaN density SHALL take the growing branch.

#### Scenario: The f32 nearest 0.35
- **WHEN** every point reads the f32 nearest 0.35
- **THEN** nothing grows, while the next f32 up grows plants
