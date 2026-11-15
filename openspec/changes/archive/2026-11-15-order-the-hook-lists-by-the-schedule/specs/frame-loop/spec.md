## ADDED Requirements

### Requirement: Scene frame list order
The system SHALL run the scene frame list's work in the original's priority order: the simulation's frame part (the step results applied and the bodies drawn between their last two steps) before the player light, and the room tracking after this frame's camera has propagated and before the live scene's frame.

#### Scenario: The rooms before the scene's frame
- **WHEN** the frame's schedules are built with the physics, player light, room and live scene plugins
- **THEN** the characters are drawn in the fixed main loop, the light is placed in `Update`, and the rooms are tracked after transform propagation and before the scene's frame

### Requirement: Unordered list work does not conflict
Where the schedule leaves two pieces of one hook list's work unordered, they SHALL NOT conflict in their data, so that either order computes the same.

#### Scenario: The game task's list
- **WHEN** the session's frame, the held item, the camera, the player light and the spirit screen are built into `Update`
- **THEN** the held item runs after the camera and no unordered pair of them conflicts

### Requirement: The player light at the drawn head
The player light SHALL be placed 80 cm over the player's head part as the frame draws it, between the part's last two body ticks at the frame's fraction, as the original's light hook follows the simulation's interpolation hook.

#### Scenario: A head between ticks
- **WHEN** the light is placed at a frame fraction of one half
- **THEN** it hangs over the head part's drawn pose at that fraction, not over its last tick's pose
