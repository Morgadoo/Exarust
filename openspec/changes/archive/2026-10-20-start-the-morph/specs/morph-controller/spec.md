## ADDED Requirements

### Requirement: Morph start setup
The morph start SHALL create the owner's helper when it has none and SHALL ensure the owner's controls only when both the helper and the controls are missing.

#### Scenario: Missing helper and controls
- **WHEN** the owner has no helper and no controls
- **THEN** the start creates the helper and ensures the controls
- **WHEN** the owner already has a helper
- **THEN** the start creates nothing and ensures nothing

### Requirement: Morph vertex binding
For each owner vertex the start SHALL transform it by the frame, bind it to the nearest candidate by the smallest squared distance from the maximum float, store that candidate's barycentric weights as the offset and the vertex as the base. With no candidate it SHALL use joint 0, a zero offset and list the vertex as unmatched.

#### Scenario: Bound and unmatched
- **WHEN** two vertices sit at 0 and 10 and two candidates sit with them, each with its own triangle
- **THEN** the records take joints 0 and 1, each with weights (1, 0, 0) and the vertex as its base, and nothing is unmatched
- **WHEN** there are no candidates
- **THEN** the record takes joint 0, a zero offset, the vertex as its base, and the vertex is unmatched
