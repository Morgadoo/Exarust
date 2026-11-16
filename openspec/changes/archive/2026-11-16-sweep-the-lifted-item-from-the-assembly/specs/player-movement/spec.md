## MODIFIED Requirements

### Requirement: Item cursor placement sweep
The system SHALL start a placed lifted item's box centre 600 cm back along the cursor ray from the placed point and move it along the ray to the first contact of any of its collision spheres within 1000 cm, the level, the props' stand-ins and the loose props' meshes met and the held body left out; a sweep that meets nothing SHALL move it the whole 1000 cm, and a sphere that starts touching SHALL count as no contact.

#### Scenario: Wall before the pick
- **WHEN** the item's sphere meets a wall 50 cm before the picked point
- **THEN** the item stops at that contact

#### Scenario: Onto the floor
- **WHEN** the cursor lies on a floor and the item's only sphere hangs 20 cm under its box centre
- **THEN** the box centre comes to rest on the ray 40 cm over the floor

#### Scenario: Against a dropped item
- **WHEN** the ray meets the front face of a dropped cube before the floor
- **THEN** the item stops with its sphere against that face

## ADDED Requirements

### Requirement: Sweep spheres start at the near plane
A sphere of the placement sweep whose far side would start in front of the camera's near plane SHALL start where its far side reaches the plane, and sweep only what is left to the best contact so far; a NaN gap SHALL not move it.

#### Scenario: A start behind the eye
- **WHEN** the sweep's start lies behind the camera
- **THEN** each sphere starts at the near plane and the item still comes to rest on the floor

### Requirement: Lift with the controls off
With the controls off, the lift SHALL be turned and placed by the GUI drop's rule (the −100000 cm limit, the same sweep, no markers, no reach test, no hover word), a fitting placement thawing a frozen item; blocked or second-blocked, the lift SHALL be neither turned nor placed.

#### Scenario: Controls off
- **WHEN** the controls are off while an item is lifted over a floor
- **THEN** it comes to rest on the floor and the hover word stays 0

#### Scenario: Blocked
- **WHEN** the controls are blocked while an item is lifted
- **THEN** it stays where the lift took it
