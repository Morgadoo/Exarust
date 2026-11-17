## ADDED Requirements

### Requirement: Fast camera shots clear the asked pitch
A fast point shot or two-point shot SHALL clear the pitch the orbit drag asked for before it places the camera; the story start's shot SHALL keep it.

#### Scenario: The practice start
- **WHEN** the practice start's fast shot runs with an asked pitch of 1.2
- **THEN** the asked pitch is 0 and the camera's pitch is the floor of its zoom's stop

### Requirement: Spectated arena start shot
The arena's spectated match start SHALL frame the spawn spot with the fast two-point shot from the spot plus 700, 1180, 1100 cm, and warp the pointer to the shot's point while the window has the focus.

#### Scenario: A spectated match begins
- **WHEN** a match with no entrant of the player's starts in a focused window
- **THEN** the free camera looks at the spot at the start zoom's stop with the asked pitch cleared, and the pointer is warped
