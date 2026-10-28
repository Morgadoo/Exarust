## MODIFIED Requirements

### Requirement: Sphere and plane
A vertex behind a level plane SHALL be pushed out to 2 cm along the outward normal. Body spheres SHALL push the cloth's triangles through the body stage and push pass (physical-body: Cloth body contact pushes triangles, Cloth push pass), not each vertex in turn.

#### Scenario: Clearance
- **WHEN** a vertex behind a horizontal plane is stepped
- **THEN** it sits 2 cm above the plane

### Requirement: Sleep
A still cloth SHALL count its sleep down, and SHALL NOT go below its owner's count. Asleep, it SHALL NOT fall. A body that pushes it SHALL wake it only when the body's countdown is higher than the cloth's.

#### Scenario: Still, then pushed
- **WHEN** a still cloth counts down to sleep, then a moving body's sphere covers it
- **THEN** it does not fall while asleep, and the push moves it and wakes it

### Requirement: Body reaction
The reaction SHALL be the opposite of the sum of the vertices' moves from this tick's collisions and push passes.

#### Scenario: One push
- **WHEN** a moving body's sphere wakes a sleeping cloth and pushes its triangle
- **THEN** the reaction is the opposite of the vertices' summed move

## REMOVED Requirements

### Requirement: Swept sphere
**Reason**: The original's body contact pushes triangles off every body sphere, standing or swept. It has no separate swept-sphere vertex push.
**Migration**: See physical-body "Cloth body contact pushes triangles" (scenario: A sweep through the face).

### Requirement: Cloth pressure
**Reason**: The record misread the push pass: it moved the normal instead of the vertex and lacked the behind-the-plane test. The live push pass replaces it.
**Migration**: See physical-body "Cloth push pass".
