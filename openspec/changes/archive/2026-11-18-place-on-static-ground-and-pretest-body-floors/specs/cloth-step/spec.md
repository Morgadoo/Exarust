## ADDED Requirements

### Requirement: Cloth sphere push floor per level mesh
A resting cloth particle's sphere contact with a level mesh SHALL use the push floor of the query form that answers for that mesh: 1e-6 for a mesh answering through its kd-tree, 1e-10 for one answering through its brute-force query.

#### Scenario: A particle a hair inside its padding
- **WHEN** a resting particle lies 4e-7 cm inside its 2 cm padding of a level face
- **THEN** it is pushed off a mesh without a kd-tree and left where it is by a mesh with one
