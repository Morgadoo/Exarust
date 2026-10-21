# Spec Delta

## ADDED Requirements

### Requirement: Idle camera turn
Each frame the player SHALL be idle when the pointer stayed within 2 pixels of where it was last seen on both axes, the condition word is 0, no foot steps, the hand holds nothing and no mouse button is held; a dead player SHALL always be idle. A busy frame SHALL note the pointer and the game clock.

#### Scenario: Busy and idle frames
- **WHEN** the pointer moves 2 pixels, then 3 pixels, and the player walks, steps, holds an object or holds a button, and a dead player's pointer jumps
- **THEN** 2 pixels stays idle, 3 pixels and the busy cases are busy, and the dead player is idle

### Requirement: Idle turn rate
Once the game clock is past the noted time plus 10 000 ms, an idle frame SHALL turn the camera's target yaw by 0.0001 rad for each millisecond since the last frame, in double precision then single, less 2π once past it, and the behaviour SHALL stop reading the cursor until the player is busy again.

#### Scenario: Ten seconds
- **WHEN** a busy frame at 1000 ms is followed by idle frames at 11 000, 11 001 and 12 001 ms
- **THEN** the yaw stays at 11 000 ms, turns 0.0001 rad at 11 001 ms and 0.1001 rad by 12 001 ms, and the cursor is no longer read
