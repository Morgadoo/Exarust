## ADDED Requirements

### Requirement: Wheel press
A drag start on a lever's arm (the wheel fitting of its operative class 3) SHALL take no link: it SHALL mark the lever held, wake it, and keep the angle between the arm and the cursor's point on the plane through the lever's node across its axle.

#### Scenario: Taking the arm
- **WHEN** the press's point is the arm point
- **THEN** the kept angle is 0 and the lever is held and awake

### Requirement: Wheel turn each frame
While the hand turns a lever and its hip is within 300 cm of the lever's node, each frame the lever's hand angle SHALL be the cursor's angle on the first plane plus the kept one, wrapped, when the camera's view lies along the axle by at least 0.2 (a double compare; a NaN takes it), else the heading of the cursor's sideways offset on the plane across the arm at 30 cm out, waking the lever.

#### Scenario: The second grip
- **WHEN** the view lies across the axle and the cursor is 5 cm sideways
- **THEN** the hand angle is atan(5/30)

### Requirement: Wheel clamp and release
The wheel's hand angle SHALL be clamped to the swing from the first detent (never below it, at most one step past it; a NaN passing) and wrapped into ±π. Out of reach the lever SHALL be let go and woken; the release SHALL let it go.

#### Scenario: Past the swing
- **WHEN** the cursor is past the far end of the swing for 200 frames
- **THEN** the hand angle is the second detent, the arm arrives there, and the lever signals 1 once

### Requirement: Push by interaction
A double click on a push surface whose operative flag 4 is clear SHALL push it and wake it while the point is in reach.

#### Scenario: A pressure plate
- **WHEN** a pressure plate carrying flag 4 is double clicked
- **THEN** it is not pushed
