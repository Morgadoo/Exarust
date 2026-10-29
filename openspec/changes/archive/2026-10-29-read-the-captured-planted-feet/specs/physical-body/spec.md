## ADDED Requirements

### Requirement: Captured planted feet
Every capture SHALL record, for each foot's bone, whether that foot touched when the capture was taken; bones never recorded SHALL read as not planted.

#### Scenario: Capture while standing
- **WHEN** a clip starts while the left foot touches and the right does not
- **THEN** the capture reads the left foot's bone as planted and the right's as not

### Requirement: Muscles-off lead-in
The muscles-off sampler SHALL blend from the capture, without ease, while its bracket aims at the capture's key, and on aiming at another key SHALL drop the capture and the blend but keep the clock.

#### Scenario: Leaving the capture's key
- **WHEN** the clock passes the first key at 30 frames with a 4-frame blend left
- **THEN** the capture and the blend are gone and the clock still reads 30

### Requirement: Muscles-off feet
A muscles-off body that was not walking SHALL anchor each untouched foot where it is. A walking body with support SHALL lift a foot whose destination key is not flagged, or whose source (the capture's flag while leading in) is not flagged at a blend of at most a half, and SHALL otherwise plant an untouched foot at an anchor height of three times its scale in cm; without support it SHALL leave its feet.

#### Scenario: Unflagged destination
- **WHEN** a walking, supported muscles-off body's destination key has no plant flag for a touching foot
- **THEN** the foot lifts and its anchor is re-taken where it is

### Requirement: Swing voice start
The swing voice SHALL start silent at 0.4 times the weapon's swing pitch on the first attack tick in the swing's region and again after leaving it, and SHALL follow the second hand's tracker for a one-hand striker's other-side attacks. The start SHALL rustle the clothing at the foley volume times 0.02 r + 0.02 and pitch 0.4 r + 0.8 when the foley clock has passed, the clock then waiting 400 ms plus a jitter under 100 ms.

#### Scenario: Entering the region
- **WHEN** a weapon of swing pitch 1.5 enters the swing's region
- **THEN** its voice starts at volume 0 and pitch 0.6

### Requirement: Held voice fade
While the body is not attacking, each held item's playing voice SHALL lose 0.01 of volume and 0.0005 of pitch a tick and be asked to stop under 0.005.

#### Scenario: Fading voice
- **WHEN** a held voice at volume 0.5 and pitch 1 fades one tick
- **THEN** it plays at 0.49 and 0.9995 and keeps playing

### Requirement: Walking rustle
A walking plant SHALL rustle the clothing at the foley base times (0.5 g + 0.5)(0.02 speed + 0.01) and pitch (0.2 p + 0.9)(0.2 speed + 0.7) divided by the body's size, the foley clock then waiting 400 ms plus a jitter under 100 ms.

#### Scenario: Five centimetres a tick
- **WHEN** a body of size 1 with foley base 2 plants walking at 5 cm a tick with both draws 0.5
- **THEN** the rustle's volume is 0.165 and its pitch 1.7
