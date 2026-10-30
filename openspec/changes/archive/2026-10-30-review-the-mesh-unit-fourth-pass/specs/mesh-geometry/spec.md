## ADDED Requirements

### Requirement: Swept-sphere travel
The swept-sphere test SHALL measure an accepted hit from the sweep's start (`closest·d − start·d − √(r² − p²)`) and SHALL use the best distance so far as the sweep's travel, so a mesh walk shortens the sweep to each accepted hit.

#### Scenario: A grazed wall after a floor
- **WHEN** a 0.5 ball sweeps 5 down from 1.5 over a floor at y = 0 and then past a wall 0.3 beside it
- **THEN** the floor hits at 1.0 and the wall, tested over the shortened travel, wins at 0.6 (4.6 when swept alone)

### Requirement: Material texture matrix
When the record shading a group has extra flag `0x40`, the GPU build SHALL write each uploaded texture coordinate as `(u·m0 + v·m3) + m9`, `(u·m1 + v·m4) + m10` with the record's 12-float matrix, while the split tangents keep the stored coordinates.

#### Scenario: Scale about the centre
- **WHEN** the matrix scales by 2 about the centre with offset (0.25, −0.5)
- **THEN** (0.5, 0.5) uploads as (0.75, 0.0)
