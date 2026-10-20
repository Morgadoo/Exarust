## ADDED Requirements

### Requirement: Morph rest pose
Restoring SHALL return the blob's base and its rest points.

#### Scenario: A stored pose
- **WHEN** the blob's base is (1, 0, 0) and its rest points are the origin and (1, 0, 0)
- **THEN** restoring returns exactly those

### Requirement: Morph weighted base
The weighted base SHALL be the blob's base plus each part's pivot times its weight, skipping a weight whose absolute value is under 1.19209e-7.

#### Scenario: Two parts and a tiny weight
- **WHEN** the pivots are (2, 0, 0) and (0, 3, 0) with weights 0.5 and 2
- **THEN** the base is (2, 6, 0)
- **WHEN** the first weight is 1e-8
- **THEN** the base is (1, 6, 0)

### Requirement: Morph points
The full evaluation SHALL start from the rest points and add each part's pivot and per-vertex delta times its weight, skipping weights under the epsilon, and SHALL return the base and the points.

#### Scenario: Two weighted parts
- **WHEN** two parts of weight 2 and 1 act on two rest points
- **THEN** the base and both points include each part's scaled pivot or delta
