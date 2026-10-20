## ADDED Requirements

### Requirement: Skin record merge and normalize
A merge SHALL add a scaled source through the four-slot joint merge. Normalization SHALL stop at the first zero joint, zero a negative weight, then divide the weights through the last used slot by the sum.

#### Scenario: Three joints and a negative weight
- **WHEN** joints 1, 2 and 3 carry weights 1, 1 and 2
- **THEN** normalization gives 0.25, 0.25 and 0.5
- **WHEN** joints 1 and 2 carry weights 2 and −1
- **THEN** normalization gives 1 and 0

### Requirement: Skin fill
A fill SHALL merge each neighbour at 1/√(distance squared) and divide by the distance sum. With no neighbours it SHALL leave the record.

#### Scenario: Two neighbours and none
- **WHEN** neighbours at squared distances 4 and 9 are merged
- **THEN** their 0.5 and 1/3 weights are divided by their sum
- **WHEN** there are no neighbours
- **THEN** the fill reports false and the record is unchanged

### Requirement: Skin relaxation
Three relaxation passes SHALL make each record half itself plus half of 1/√(distance squared) times each neighbour, then normalize.

#### Scenario: Two vertices
- **WHEN** two vertices at squared distance 4 relax toward each other
- **THEN** each record holds both joints, the self weight is larger, and the weights sum to 1
