# Spec Delta

## ADDED Requirements

### Requirement: Cube faces follow a moved light
When a light moves, the system SHALL work out again, from the light's new position, the cube faces each of its casters touches, so a face draws every caster it holds.

#### Scenario: Walking with a torch
- **WHEN** the player walks past crates with a carried torch
- **THEN** the crates' shadows are whole, not cut along a cube face's edge
