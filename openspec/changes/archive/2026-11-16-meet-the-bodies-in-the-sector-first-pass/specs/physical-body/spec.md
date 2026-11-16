## ADDED Requirements

### Requirement: Sector first pass
Before any character ticks, the sector step SHALL walk its simulated objects in list order, and each whose class has the first-pass bit and that is not asleep, disabled or removed SHALL be offered the body parts linked in the nine grid cells around its own cell, its own first. A part SHALL be offered when its bound sphere is not apart from the object's, the object's centre lies within the object's radius of the part's mesh box (a NaN passing both) and its body is not disabled.

#### Scenario: A disabled body
- **WHEN** a thrown blast flies at a body that is disabled
- **THEN** the body is not offered and does not move

### Requirement: Particle body stage
A particle system with the solver's body bit SHALL meet the bodies in the solver's object stage, after body-body detection, each part of the nine cells around its own passing the sector's gates (a sleeping body too) except the part on its own node's parent, and SHALL hand each part met its dose through the part's dose slot.

#### Scenario: The emitter's own part
- **WHEN** an emitter whose node hangs from a part meets that part
- **THEN** the part is skipped and takes no dose call
