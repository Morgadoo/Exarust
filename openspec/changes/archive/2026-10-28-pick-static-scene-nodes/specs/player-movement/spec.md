## MODIFIED Requirements

### Requirement: Pick under the cursor
The pick SHALL look 40 m along the cursor's ray at the level and the props' meshes, naming a prop's body and part by its mesh, and at the characters' collision spheres, the nearest hit first. A static mesh built under a named scene node SHALL be named as that node, and other static geometry SHALL stay the level; the hand SHALL treat a scene node as the level.

#### Scenario: The nearest character
- **WHEN** a dead character's sphere 3 m ahead and a living one's 6 m ahead are on the ray, with nothing fixed
- **THEN** the pick hits the dead one at 2.8 m and names a corpse, and a ray beside them hits nothing

#### Scenario: A scene node's mesh
- **WHEN** a unit cube under a scene node sits at 1 m height and another cube with no node at 2 m along z, and rays fall onto each from above
- **THEN** the first collider names the node and the second names none

## ADDED Requirements

### Requirement: Pick-only geometry
A model's `_null` groups SHALL be built as static colliders that only the cursor's pick meets; the character controller, the bodies' level contacts, sweeps and the other rays SHALL pass through them.

#### Scenario: A pick-only box
- **WHEN** a pick-only unit cube under a scene node sits at 1 m height and rays and a 10 cm ball fall onto it from 3 m
- **THEN** the cursor pick meets it 1.5 m down and names the node, and the plain ray, the filtered pick, the sweep and the level pieces find nothing
