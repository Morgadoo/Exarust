## ADDED Requirements

### Requirement: Morph basis
Solving the morph basis SHALL project the query point onto the control triangle's plane through its first point along the stored normal, then return the triangle's barycentric weights: the branch SHALL be chosen by the absolute x sum against the absolute y sum, the first two weights SHALL come from that branch, and the third SHALL be one minus them.

#### Scenario: First and second branches
- **WHEN** the triangle is (0, 0, 0), (1, 0, 0), (0, 1, 0), its normal is (0, 0, 1) and the query is (0.25, 0.25, 5)
- **THEN** the weights are 0.5, 0.25 and 0.25
- **WHEN** the triangle is (0, 0, 0), (2, 0, 0), (1, 0, 1), its normal is (0, 1, 0) and the query is (0.5, 5, 0.25)
- **THEN** the weights are 0.625, 0.125 and 0.25
