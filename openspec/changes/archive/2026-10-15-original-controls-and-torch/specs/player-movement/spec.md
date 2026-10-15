## ADDED Requirements

### Requirement: Tank controls
The system SHALL steer the player in exploration as the original does. W SHALL walk along the character's heading; A and D SHALL turn it by 0.025 rad per 83 Hz tick while moving, or by a quarter turn (at most once a second) when standing. S SHALL brake when faster than 1 cm per tick, then backpedal. Neither the camera nor the cursor SHALL change the heading.

#### Scenario: Walking forward
- **WHEN** W is held while the camera is orbited
- **THEN** the player keeps walking the same way

#### Scenario: Turning in place
- **WHEN** D is pressed while standing
- **THEN** the player turns a quarter to the right

### Requirement: Walk to the cursor
The system SHALL walk the player toward the cursor's point on the hip-height plane while the right mouse button is held, running when that point is more than 3 m away.

#### Scenario: Far cursor
- **WHEN** the right button is held over a point 5 m away
- **THEN** the player runs toward it

### Requirement: Combat footwork
The system SHALL toggle combat mode on Tab. In combat mode the character SHALL face the cursor, and W, S, A and D SHALL step along and across that direction.

#### Scenario: Side step
- **WHEN** A is held in combat mode
- **THEN** the player steps to its left while still facing the cursor

### Requirement: Carried torch
The system SHALL give the player the game's torch in the left hand in play mode, lit, and SHALL use the "1H & Torch" moveset while it is carried.

#### Scenario: Starting a level
- **WHEN** play mode starts in a dungeon
- **THEN** the player holds a burning torch that lights and shadows the room

## MODIFIED Requirements

### Requirement: Chase camera
The system SHALL keep the camera 20 m from the player, following its position only, and SHALL zoom by field of view (zoom / 2000 × 45° × √(1.6 / aspect), six wheel stops from 800 to 2000). The pitch SHALL follow the zoom (0.8 to 0.925 rad below the horizon) unless the middle-mouse orbit drag raises it, up to 1.5 rad. The camera SHALL never turn by itself in exploration.

#### Scenario: Zoom
- **WHEN** the mouse wheel is turned
- **THEN** the field of view steps between 18° and 45° while the distance stays 20 m

## REMOVED Requirements

### Requirement: Camera-relative movement
**Reason**: The original has no camera-relative keyboard movement; W walks the character's heading and A/D steer it.
**Migration**: See Tank controls and Walk to the cursor.
