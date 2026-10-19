# Spec Delta

## ADDED Requirements

### Requirement: Sphere and plane
A vertex inside a body sphere SHALL be pushed out to the radius plus 2 cm. A higher friction SHALL leave less tangential motion. A vertex behind a level plane SHALL be pushed out to 2 cm along the outward normal.

#### Scenario: Clearance
- **WHEN** a vertex at a sphere's centre of radius 10 cm is stepped, then one behind a horizontal plane
- **THEN** the first ends 12 cm from the centre and the second sits 2 cm above the plane

### Requirement: Sleep
A still cloth SHALL count its sleep down, and SHALL NOT go below its owner's count. At zero it SHALL NOT fall. A push SHALL wake it.

#### Scenario: Still, then pushed
- **WHEN** a still cloth counts down to sleep, then a sphere covers it
- **THEN** it does not fall while asleep, and the push moves it and wakes it

### Requirement: Rustle and mesh
Rustle SHALL play when the squared sum of the velocities exceeds 400, and its envelope SHALL decay by 0.9 when the cloth is quiet. The mesh vertices SHALL be the particle positions.

#### Scenario: Fast cloth
- **WHEN** a fast cloth is stepped, then a quiet one, and a falling vertex is read as a mesh
- **THEN** rustle plays and then decays, and the mesh matches the moved vertex
