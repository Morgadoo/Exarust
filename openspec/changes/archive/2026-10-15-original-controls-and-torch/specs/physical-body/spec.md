## ADDED Requirements

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL move at most 25 m/s, a single contact push SHALL be at most twice the sphere's radius, and a fallen body SHALL align its targets to its pelvis's position and rotation only, without scale.

#### Scenario: Deep contact
- **WHEN** a body's sphere is found deep inside level geometry
- **THEN** it is pushed out at most twice its radius per contact pass, and no particle gains more than 25 m/s
