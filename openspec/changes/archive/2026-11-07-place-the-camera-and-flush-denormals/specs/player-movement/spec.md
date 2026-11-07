## ADDED Requirements

### Requirement: Game camera preset
The chase camera SHALL take its numbers from the game camera's preset record as the original's preset applies it: the yaw spring's stiffness SHALL be the preset view over 59² (250/3481) and its retention the 59th root of the default 3.94e-13, the zoom SHALL start at the preset's first distance 1300 between its smallest and largest distances 800 and 2000, and the shown zoom SHALL ease 5/59 of the way a tick.

#### Scenario: The spring constants
- **WHEN** the chase camera is built
- **THEN** its stiffness is 250/3481 in single precision and its retention 0.6162461

### Requirement: Camera shot placement
A scripted camera shot SHALL set the camera's target and shown yaw to the engine's planar yaw (its fast arc cosine) of the vector from the target to the eye and stop the orbit. A shot with a distance SHALL set the shown zoom to it and the zoom to it held between the smallest and largest distances; a shot without one SHALL keep the zoom.

#### Scenario: The story start
- **WHEN** a shot puts the eye 600, 600, 1300 cm from the player with distance 1300
- **THEN** the yaw is the planar yaw of that offset and the zoom is 1300

#### Scenario: A distance past the stops
- **WHEN** a shot's distance is 5000
- **THEN** the zoom is 2000 and the shown zoom 5000
