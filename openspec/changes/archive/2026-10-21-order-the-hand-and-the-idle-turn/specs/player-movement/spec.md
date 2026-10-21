# Spec Delta

## MODIFIED Requirements

### Requirement: Cursor pick
A click SHALL name a free hinge as a door, a dead character as a corpse and a living one as a person. A loose prop whose mobility is under 0.08 SHALL be ignored unless it is tagged bulky, and a prop needing strength SHALL be ignored when strength is under 0.9, both compared in double precision. A usable item on a prop SHALL stay usable, and a usable item with no body SHALL be nothing. A point farther than 70 cm SHALL be turned toward first.

#### Scenario: A door, a light prop, and the reach
- **WHEN** a free hinge, a blocked hinge, a prop of mobility 0.07, a prop of mobility 0.08, a prop of mobility 0.080001, a bulky prop of mobility 0.07, a usable prop, that usable prop with its body removed, a point 70 cm away and a point 70.1 cm away are read
- **THEN** the names are a door, nothing, nothing, nothing, a loose prop, a loose prop, usable, nothing; 70 cm does not turn and 70.1 cm does

#### Scenario: Strength
- **WHEN** a prop needing strength is read with strength 0.89, 0.9 and 0.900001
- **THEN** the first two are nothing and the third is a loose prop

### Requirement: When the reach is measured
The reach SHALL be measured each frame: outside combat mode while the right button is not walking to the cursor, and in combat mode only with Ctrl held or something held, whatever the right button does. A drag in hand SHALL be within reach. The player down, collapsing or dead SHALL let go of everything and be out of reach. Outside combat mode the cursor SHALL show the pointer within reach and the cross beyond it.

#### Scenario: A refused press and the cursor
- **WHEN** a press is refused outside combat mode over a point within reach while walking to the cursor
- **THEN** the cursor shows the cross

### Requirement: Idle camera turn
Each frame the player SHALL be idle when the pointer stayed within 2 pixels of where it was last seen on both axes, the condition word is 0, no foot steps, the hand holds nothing and no mouse button is held; a dead player SHALL always be idle. A busy frame SHALL note the pointer and the game clock. The test SHALL follow the hand's frame, and the frame's systems SHALL be ordered without a cycle.

#### Scenario: Busy and idle frames
- **WHEN** the pointer moves 2 pixels, then 3 pixels, and the player walks, steps, holds an object or holds a button, and a dead player's pointer jumps
- **THEN** 2 pixels stays idle, 3 pixels and the busy cases are busy, and the dead player is idle

#### Scenario: The frame's order
- **WHEN** the frame's schedule of the player, the cursor, the hand, the idle turn, the hotkeys and the player light is built
- **THEN** it builds
