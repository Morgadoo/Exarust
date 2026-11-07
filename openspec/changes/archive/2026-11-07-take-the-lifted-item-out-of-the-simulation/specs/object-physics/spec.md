## ADDED Requirements

### Requirement: Lifted item out of the simulation
While the item cursor carries an item, its body SHALL be asleep, disabled and at sleep countdown 0, and each placement SHALL move it whole so its grab point lands on the placed point; the release SHALL clear the disabled state.

#### Scenario: The release count
- **WHEN** a lifted item is let go in reach
- **THEN** its countdown is 128, and a sleeping body touching its old place takes 128

### Requirement: Lift ray meets dropped items
The lift's placement ray SHALL meet the props with no stand-in collider at their meshes' nearest front face, the held item left out.

#### Scenario: Setting an item on a dropped one
- **WHEN** the cursor carries a cudgel over another dropped cudgel
- **THEN** the ray's hit is the dropped cudgel, nearer than the floor

### Requirement: Space query
The "is the space free" query SHALL report a body at given frames as meeting another when, for a part of each (another body, not disabled, its bound sphere reaching the other's mesh sphere, no conflicting layer channel), the overlap-only body test and the overlap pair test (the lower key's spheres against the other's mesh at their own radius, NaN distances passing the pre-tests) find a touch; the item cursor SHALL refuse a marker where it meets one.

#### Scenario: A crate into another
- **WHEN** a crate is placed 0.2 m into another crate's box
- **THEN** the query reports the space taken; placed 0.1 m clear, it does not
