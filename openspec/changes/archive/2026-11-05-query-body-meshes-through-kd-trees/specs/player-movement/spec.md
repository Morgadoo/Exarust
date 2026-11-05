## ADDED Requirements

### Requirement: Character limb pick
The cursor's pick SHALL meet a character limb that has its node mesh tree at the nearest front face along the ray, taken into the limb's frame, within the best distance so far. A limb without one SHALL be met by its collision spheres.

#### Scenario: A limb's cube
- **WHEN** a ray along z from the origin meets a limb whose mesh is a 10 cm cube centred 3 m away, and a parallel ray passes 12 cm to its side through its 20 cm sphere
- **THEN** the first picks the limb at 2.95 m and the second picks nothing
