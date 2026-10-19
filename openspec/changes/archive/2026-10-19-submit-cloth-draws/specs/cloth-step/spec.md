# Spec Delta

## ADDED Requirements

### Requirement: Draw submission
A double-sided cloth's submitted passes SHALL become two draw materials, the first culling the front and the second the back. A single-sided cloth SHALL become one material that culls the back.

#### Scenario: Front then back
- **WHEN** a mesh cloth is built and its draw materials are taken from a plain surface, then the cloth is marked single sided and the materials are taken again
- **THEN** the first pair culls the front and then the back, and the second list is one material that culls the back
