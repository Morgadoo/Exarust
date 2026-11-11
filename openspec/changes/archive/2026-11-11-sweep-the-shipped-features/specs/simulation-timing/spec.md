## ADDED Requirements

### Requirement: Orphan link trace
With `EXA_ORPHAN_TRACE` set, the frame's world and the simulation's SHALL each log every `ChildOf` inserted with a parent that world does not hold, naming the child, the parent, the side and the child's components.

#### Scenario: A frame-only parent
- **WHEN** the frame parents a bridged entity to a node the simulation does not hold
- **THEN** the simulation's side logs the orphan link with the child's components
