## ADDED Requirements

### Requirement: Caster face mask
The system SHALL give each cube-shadow caster the faces its bounding sphere touches as the original masks a link: a face is left out only when the sphere lies farther than its radius beyond one of the face's four side planes through the light, at 45° between the face's axis and its neighbours'.

#### Scenario: A sphere behind the light
- **WHEN** a sphere of radius 0.9 m lies 1 m behind a light along −X
- **THEN** it touches all six faces

#### Scenario: A point on a cube edge
- **WHEN** a sphere of radius 0 lies on the +X/+Y edge direction of a light
- **THEN** it touches the +X and +Y faces only
