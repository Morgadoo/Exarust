## MODIFIED Requirements

### Requirement: Chase camera
The system SHALL keep the camera 20 m from the player's pelvis, drawn between its last two body ticks as its mesh is (the player's feet while it has no body), and SHALL zoom by field of view (zoom / 2000 × 45° × √(1.6 / aspect), six wheel stops from 800 to 2000). The pitch SHALL follow the zoom (0.8 to 0.925 rad below the horizon) unless the middle-mouse orbit drag raises it, up to 1.5 rad. The camera SHALL never turn by itself in exploration.

#### Scenario: Zoom
- **WHEN** the mouse wheel is turned
- **THEN** the field of view steps between 18° and 45° while the distance stays 20 m

#### Scenario: Walking bob
- **WHEN** the player walks
- **THEN** the camera rises and falls with the pelvis, centred on it in every frame

## ADDED Requirements

### Requirement: Camera ticks
The camera SHALL tick every 17 ms of real time, a tick running once the frame passes its start, at most nine a frame with the rest of a backlog dropped. Each frame SHALL show the backward vector (renormalised), the zoom and a shake offset between the last two ticks, by how far the frame is into the last tick; the shake offset SHALL move the eye only, and only while the last tick's squared exceeds 1e-5 cm².

#### Scenario: A stalled frame
- **WHEN** a frame comes a second after the last
- **THEN** the camera runs nine ticks and shows the last of them

#### Scenario: Between two ticks
- **WHEN** a frame falls halfway into a tick
- **THEN** the zoom shown is halfway between the two ticks' zooms

### Requirement: Camera shake
The camera SHALL keep a shake offset and the offset before its last tick. A kick SHALL add its vector to the offset and subtract it from the one before. Each camera tick SHALL carry the offset on: while its change from the one before, squared, exceeds 1e-7 cm² or its own square exceeds 1e-4 cm², the one before SHALL become the offset and the offset (offset + 0.96 × change) × 0.99; otherwise both SHALL be zero.

#### Scenario: A 1 cm kick
- **WHEN** the camera is kicked by 1 cm and ticks once
- **THEN** its shake offset is 2.8908 cm and the one before 1 cm

#### Scenario: The wobble dies away
- **WHEN** a 1 cm kick is left for 6 s of camera ticks
- **THEN** the offset has swung back through zero and is zero
