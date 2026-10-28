## ADDED Requirements

### Requirement: Ground pick
The ground pick SHALL look 40 m along the unit screen ray, grow each pickable node's bounding sphere by 5 cm when the caller accepts the node and by 1 mm otherwise, and ask the node's mesh from where the ray crosses the grown sphere's top for the rest of the reach; a crossing that is not positive SHALL become zero (the mesh asked from the eye with the whole reach). Each answering node SHALL shorten the reach and become the pick; with none the pick SHALL record the ground plane.

#### Scenario: An eye under the grown top
- **WHEN** an eye 0.3 m above a floor looks straight down inside an accepted node of radius 0.5 m centred on the floor
- **THEN** the ground pick hits at 0.3 m

#### Scenario: A taken node from above
- **WHEN** a ray from 10 m up looks straight down at an accepted node of radius 0.5 m about a floor at height 0
- **THEN** the pick hits face 0 at 10 m

### Requirement: Stepped wheel zoom
The stepped camera's wheel SHALL read its zoom as a stop level (the zoom less its least over a fifth of its range, rounded to the nearest with ties to even), subtract the wheel's units over 120, hold the level between 0 and 5 by the processor's max then min, and write the level back over the stop. The chase camera's wheel SHALL use it with one notch per scroll line and 24 pixels per notch.

#### Scenario: One notch in from the third stop
- **WHEN** a zoom of 1280 in an 800–2000 range takes 120 wheel units
- **THEN** it becomes 1040

### Requirement: Zoom slide and pitch offset
A vertical zoom slide SHALL clamp `dy · 0.001 + base` to 0–1, square it and multiply it into the range above the least zoom. A non-zero pitch amplitude SHALL give an offset `exp(3 · ln(1 − (zoom − min)/(max − min))) · amplitude` with the engine's fast logarithm and exponential, which steepens the pitch only.

#### Scenario: Halfway slide
- **WHEN** a slide of 500 pixels from base 0 runs over an 800–2000 range
- **THEN** the zoom is 1100

### Requirement: Free-camera drag
An absolute drag SHALL start from the shown yaw and minus the engine's arc sine of the view's backward height, move both by the rate times the sensitivity times the pixel delta, hold the pitch's negation below the low bound first and above the high bound second (a NaN passing), and on release snap a quarter turn from the shown yaw in the moved direction when the rounded count of quarter turns has non-zero low 32 bits and is positive.

#### Scenario: A released drag
- **WHEN** a drag is released 0.9 rad from a shown yaw of 0
- **THEN** the yaw becomes π/2

### Requirement: Editor grab drag
The grab drag SHALL keep the grabbed node's ground position at the target's height, reach the node's half extent plus the zoom, turn by half the rate per pixel, raise the grabbed point by `dy · zoom / (width · aspect / 2)` and put the looked-at point at the raised point plus the camera axis at pitch zero times the reach.

#### Scenario: A raised grab
- **WHEN** a grab with zoom 10 in a 1600-pixel viewport of aspect 1.6 moves 100 pixels down
- **THEN** the point and the looked-at point rise by 100 · 10 / 1280

### Requirement: Preset editor scan
Under its editor flag a preset SHALL scan the scene for nodes of class 0xA9, each replacing the default 800 by 0 by 1000 target box with its centre and its three half extents, a 0xAE marker raising the extra flag; a first distance below the least distance SHALL be raised to it, and a first distance that is not positive, or is NaN, SHALL derive the placement's distance.

#### Scenario: The last 0xA9 node wins
- **WHEN** the scene holds two 0xA9 nodes, the last with half extents (8, 9, 10) and the marker
- **THEN** the target box takes (8, 9, 10) and the flag is raised
