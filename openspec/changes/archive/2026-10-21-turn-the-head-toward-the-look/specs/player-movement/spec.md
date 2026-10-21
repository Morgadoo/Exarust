## ADDED Requirements

### Requirement: Head look
While not helpless and on a clip under action 0x2C, a body's head SHALL turn toward its look (the smoothed cursor moved toward the head's physical forward by 1 − E², in the body's frame, its height halved) by a weight rising 0.01 a tick to 1 (0 otherwise): each angle moved by the weight toward the look's (yaw within ±1.2566 rad × 0.8, followed at most π/100 rad a tick; pitch −asin of the height; roll a tenth of the yaw).

#### Scenario: Looking aside
- **WHEN** the look is 0.5 rad aside of the frame at full weight
- **THEN** the head's yaw moves π/100 a tick and settles at 0.40024

#### Scenario: Ramping in
- **WHEN** a body starts looking
- **THEN** its weight is 0.01 after a tick and 1 by its 120th

### Requirement: Looking head anchors
While the head looks, its link muscles' anchors SHALL be placed relative to the thorax's physical transform rather than its pose.

#### Scenario: A turned pose
- **WHEN** the whole pose is turned 0.5 rad about the thorax and the head looks
- **THEN** the head's anchors take the turn, which they do not when it does not look
