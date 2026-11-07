## MODIFIED Requirements

### Requirement: Cursor Look
The orbit binding and the Cursor Look binding SHALL be one mouse capture: holding either SHALL capture the mouse, hide the system and world cursors and put the pointer back at the window's centre every frame. The capture's kind SHALL be fixed when it begins (Cursor Look's when its binding is down then, else the orbit's) and SHALL last until both are let go. Losing window focus SHALL drop it without a warp.

#### Scenario: Hide the cursors
- **WHEN** Cursor Look is held
- **THEN** neither the system pointer nor the world cursor is drawn

#### Scenario: One capture
- **WHEN** Cursor Look is held, the middle button is pressed too and Cursor Look is let go
- **THEN** the capture stays Cursor Look's until the middle button is let go

### Requirement: Orbit release snap
The end of the controls' mouse capture, the orbit's or Cursor Look's, SHALL apply the free-camera drag's release snap to the target yaw against the shown yaw.

#### Scenario: A flick past an eighth of a turn
- **WHEN** the orbit button is released with the target yaw 0.9 rad ahead of a shown yaw of 0
- **THEN** the target yaw becomes π/2

## ADDED Requirements

### Requirement: Cursor Look turn
Cursor Look SHALL turn its target, which starts at the cursor follower's target, by 3.6 × Δx / window width radians for horizontal mouse motion after its first frame, make it the cursor follower's target and the raw cursor direction, and align the camera's target behind the smoothed cursor direction every held frame. The orbit SHALL drag the camera from the frame after it began. While either holds, the cursor's point SHALL NOT be read.

#### Scenario: Turn with the mouse
- **WHEN** Cursor Look is held and the mouse moves 800 logical pixels right across a 1600-pixel window after the first frame
- **THEN** the look's target turns 1.8 radians and the behaviour's raw cursor direction is it

### Requirement: Capture release warp
The capture's end SHALL warp the pointer, with the OS layer's hold, to the screen point of the aim point (80 cm over the player's ground along the smoothed cursor direction at the cursor's last distance), or back to where it was when the capture began over the GUI.

#### Scenario: Release over the GUI
- **WHEN** a capture that began over the GUI is released
- **THEN** the pointer returns to its earlier position

### Requirement: Pointer pin while the camera aligns
While neither capture binding is held and the camera still swings to Cursor Look's alignment (until its shown yaw is within 0.01 rad of the target, compared in double precision, or an orbit drag), the pointer SHALL be moved each frame to keep the flat offset from the player of its point on the aim plane under the previous frame's view.

#### Scenario: The alignment ends
- **WHEN** the camera is aligned to a yaw 1 rad from its shown yaw and its spring settles within 0.01 rad
- **THEN** the alignment flag clears and the pointer is no longer moved
