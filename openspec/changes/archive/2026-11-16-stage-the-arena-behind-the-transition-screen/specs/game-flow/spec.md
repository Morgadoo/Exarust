## ADDED Requirements

### Requirement: A staging transition pauses its frames at black
A transition whose callback is a staging (the next practice fight, the arena's next round, the match start) SHALL raise the pause counter at the callback when it is zero, and every transition SHALL lower it, when above zero, on the screen's second frame after the callback. While it is raised the lockstep fixed loop SHALL not run, and the frames it was held SHALL not be caught up.

#### Scenario: The pause flags
- **WHEN** a next-round transition reaches black and runs four more frames, and a revive's transition reaches black
- **THEN** the first raises the pause at black and lowers it on the third frame only; the revive raises nothing

#### Scenario: The fixed loop held
- **WHEN** the pause is raised for three frames and lowered
- **THEN** no tick runs in those frames, and the next frame runs one frame's ticks

### Requirement: The transition screen's close ends the staging freeze
The transition screen's close SHALL post the staging freeze's release for the next fixed step, after any staging posted before it; a staging step run while a transition screen is up SHALL hold the freeze instead of counting it.

#### Scenario: Held and released in order
- **WHEN** a held staging and then a release are posted and the fixed queue runs after each
- **THEN** the freeze is held after the first and clear after the second

### Requirement: The match start holds the screen over its load
The match start's staging step SHALL count as a level change: the screen SHALL stay black, and its frames after the callback SHALL wait, until the change is done; the phase SHALL come back to the one the screen opened over.

#### Scenario: A loading match start
- **WHEN** a match start's transition reaches black and five loading frames follow, then three frames without the load
- **THEN** it stops the music at black, stays black and lowers nothing while loading, and lowers the pause on the third frame after
