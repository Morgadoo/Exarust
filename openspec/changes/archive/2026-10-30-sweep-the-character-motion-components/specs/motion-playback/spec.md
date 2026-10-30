# Spec Delta

## ADDED Requirements

### Requirement: Skin frame matrices
Each joint's skin matrix SHALL be its inverse bind applied first and then the joint matrix placed in the mesh node's frame (each joint row and the joint's translation less the node's dotted with the node's axis rows), unless the mesh marks its joints as already local.

#### Scenario: Base before joint
- **WHEN** the inverse bind translates by (1, 0, 0) and the local joint turns 90° about z
- **THEN** the skin matrix's translation is (0, 1, 0)

### Requirement: Skin bounds
A skinned mesh's bounds SHALL grow from the root's centre by each part's centre placed in the node's frame plus or minus its projected extent, capped by the part's own radius with ordered compares, and the middle SHALL be `min + (max − min)·½`.

#### Scenario: Middle of a small box
- **WHEN** the box runs from 0.1 − 0.2 to 0.3 along x
- **THEN** its middle is exactly 0.1 in single precision

### Requirement: Previous position priming
The previous-position stream SHALL be filled per index of the part's stream with the position-blended skin of that index's vertex; a vertex whose first joint number is below 1 or past the matrices SHALL have it set to 1 and its slot left unchanged.

#### Scenario: A joint past the matrices
- **WHEN** a vertex names joint 2 with one matrix
- **THEN** its joint becomes 1 and its slot keeps its old value

### Requirement: Motion part transfer
The role copy SHALL stop at a new part whose controller does not take copies; a source part without a controller SHALL clear the new part's node word and still pass its children on; each source child SHALL claim the first new child of its role whose controller takes copies and whose id is below 1, giving it the source child's id and the parent controller's link. The id walk SHALL number only parts with a numbered controller and descend only below them.

#### Scenario: A source part without a controller
- **WHEN** a source child of role 8 has no controller and the new tree has a role-8 child with id 0
- **THEN** that child takes the source's id, keeps its state and has its node word cleared
