## ADDED Requirements

### Requirement: Sun shadow map size
The system SHALL give the sun one 4096² shadow map, the original's at its top shadow detail.

#### Scenario: Any sunlit level
- **WHEN** a level with a sun is shown
- **THEN** the sun's shadow map is 4096 texels on a side

### Requirement: Fitted sun map in terrain levels
In a level with a terrain the system SHALL replace the sun's cascades for each active camera with one orthographic map fitted to that camera's view; other levels SHALL keep two cascades over 60 m, and so SHALL every level when the fit is turned off.

#### Scenario: A terrain level
- **WHEN** an outdoor level with a terrain is shown
- **THEN** the sun has one shadow cascade per camera, sampled out to 125 m

#### Scenario: A level without a terrain
- **WHEN** a sunlit level without a terrain is shown
- **THEN** the sun keeps two cascades over 60 m

### Requirement: Sun fit clipping planes
The sun fit SHALL clip the camera's view volume, out to 125 m, per axis by the sign of the camera's backward axis: a negative component SHALL keep the axis at or below the high edge, a positive one at or above the low edge, and a zero or NaN one SHALL add no plane. On x and z the edges SHALL be the level box's centre plus and minus its half extent; on y they SHALL be the level's ceiling and floor when it has an environment record, and the box's x edges otherwise.

#### Scenario: Looking down and away
- **WHEN** the camera's backward axis is (0.5, 0.7, −0.5) in a level box from (−1000, −200, −500) to (3000, 400, 1500) cm with floor 0 and ceiling 340 cm
- **THEN** the volume keeps x ≥ −1000, y ≥ 0 and z ≤ 1500 cm

#### Scenario: No environment record
- **WHEN** that level has no environment record
- **THEN** the vertical limits are 3000 and −1000 cm

### Requirement: Sun fit radius steps
The fitted half extent SHALL be 200·round(r/200 + 0.4) cm, the quotient in single precision, the bias added in double precision and rounded to nearest with ties to even; it SHALL change only when that value differs and the radius is more than 60 cm above (grow) or more than 20 cm below (shrink) the current half extent, which SHALL start at 3000 cm and carry from frame to frame.

#### Scenario: A grow held back
- **WHEN** the half extent is 3000 cm and the radius is 3060 cm
- **THEN** the half extent stays 3000 cm

#### Scenario: A grow taken
- **WHEN** the half extent is 3000 cm and the radius is 3061 cm
- **THEN** the half extent becomes 3200 cm

#### Scenario: A shrink
- **WHEN** the half extent is 3000 cm and the radius is 2819 cm
- **THEN** the half extent becomes 2800 cm

### Requirement: Sun fit centre and depth
The fit SHALL rotate the sphere's centre into the light's frame (right = direction × +Y, up = right × direction, a fixed frame past |y| 0.9999), snap its x and y to multiples of 2·half extent / 4096 rounded to nearest with ties to even, and give the box ±8000 cm along the light.

#### Scenario: Snapping
- **WHEN** the half extent is 1000 cm and the centre's light-space x is 10 cm
- **THEN** x becomes 20 texels of 0.48828125 cm

### Requirement: Sun fit with too few corners
When fewer than two corners of the clipped view survive (merged within 0.1 cm²), the system SHALL keep the previous frame's fitted map.

#### Scenario: The view leaves the level
- **WHEN** the level box lies wholly behind the camera
- **THEN** the sun's map stays as the last fitted one
