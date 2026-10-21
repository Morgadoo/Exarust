# Spec Delta

## ADDED Requirements

### Requirement: Camera yaw wrap
Cursor Look SHALL keep the camera's target yaw within half a turn of the camera's present yaw, adding or subtracting a whole turn when the difference, compared in double precision, exceeds π.

#### Scenario: Wrapping the target
- **WHEN** the camera has orbited to a yaw of 7 rad and Cursor Look asks for a target yaw of 0.25 rad
- **THEN** the target becomes 0.25 + 2π

### Requirement: Camera pitch and orbit rate
The pitch floor SHALL be 0.8 rad below zoom 1000 and 0.000125 rad steeper per unit above, the zoom less 1000 in single precision and the rest in double. The camera SHALL hold the asked pitch, or the floor when that is steeper. The orbit SHALL turn 4.8 rad over the screen width, divided in double precision.

#### Scenario: Floor and held pitch
- **WHEN** the zoom is 1000.5 and the asked pitch is 0.5 rad
- **THEN** the floor is 0.8000625 rad and the camera holds it

### Requirement: Free zoom
A free zoom SHALL move 0.2 % of its value per wheel unit (120 to a notch), worked in double precision, but at least 0.5 % of its range, and SHALL stay in its range.

#### Scenario: A wheel notch
- **WHEN** a free zoom of 5000 in a 10–10000 range turns one notch in
- **THEN** it becomes 3800

### Requirement: Camera placement
A placement without a distance SHALL measure the eye from the looked-at point before its height clamp, scale the vector by its reciprocal length, and take the engine's planar yaw and arc sine of it.

#### Scenario: A clamped point
- **WHEN** the point (1, 5, 2) is clamped to a height of 10 and the eye is at (1, 20, 7)
- **THEN** the backward vector is (0, 15, 5) over its length and the yaw is 0

### Requirement: Camera preset
A preset SHALL fill its defaults: 280 and 16000 over 59², a retention of 3.94e-13 per second whose 59th root is 0.6162, pitch bounds of ±1.5 and distances of 10 to 10000. The stepped camera SHALL take speeds of 5 and 5/59; any other camera 15 × its largest distance / 59² and 6.6/59.

#### Scenario: The default preset
- **WHEN** a preset with every field zero is applied
- **THEN** its retention root is 0.6162 and its view scale 280 / 3481

### Requirement: Camera kick offset
A kick SHALL add its direction, normalised, to a small random offset and scale the sum to its size; a direction shorter than √0.1 SHALL be replaced by the camera's right axis times the sign of a random draw, lifted by a second draw.

#### Scenario: A degenerate direction
- **WHEN** a zero direction is kicked and the fourth draw is below a half
- **THEN** the basis is the camera's left

### Requirement: Cut-away segment
The cut-away segment SHALL start twice the camera's largest distance beyond the looked-at point along the backward vector and run back towards the point for twice that distance less 1 m.

#### Scenario: The segment
- **WHEN** the looked-at point is at height 10 m, the backward vector is +Z and the largest distance 20 m
- **THEN** the segment starts at z = 40 m and ends at z = 1 m

### Requirement: Mesh pick
The mesh pick SHALL visit nodes whose class matches 0x89 without the no-pick class bit and whose layers meet the view's. The segment, shortened by each hit, SHALL meet the node's bounding sphere and then its box, and each mesh SHALL be asked for a face within the best distance so far, keeping the nearest. With no hit the pick SHALL record where the ray meets the ground plane.

#### Scenario: A hit shortens the segment
- **WHEN** a mesh is met 1 m along and a later node's sphere lies 3 m along with a radius of 0.5 m
- **THEN** the later node is not asked

### Requirement: Kinesis pick
The kinesis pick SHALL need the node's kinesis option bit, look 40 m, grow each sphere by the caller's height and ask the mesh from where the ray crosses the grown sphere's top.

#### Scenario: Kinesis from above
- **WHEN** a ray from 10 m up looks straight down at a kinesis node of radius 1 m over a floor at height 0, with a height of 2 m
- **THEN** the pick hits at 10 m
