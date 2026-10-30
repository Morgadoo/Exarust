## MODIFIED Requirements

### Requirement: Preset editor scan
Under its editor flag a preset SHALL scan the scene for nodes of class 0xA9 whose class word's low twelve bits are 1, each replacing the default 800 by 0 by 1000 target box with its centre and its three half extents, such a node's 0xAE marker raising the extra flag; a first distance below the least distance SHALL be raised to it, and a first distance that is not positive, or is NaN, SHALL derive the placement's distance.

#### Scenario: The last 0xA9 node wins
- **WHEN** the scene holds two 0xA9 nodes with class word 1 and 0x1001, the last with half extents (8, 9, 10) and the marker
- **THEN** the target box takes (8, 9, 10) and the flag is raised

#### Scenario: A wrong class word
- **WHEN** a 0xA9 node with the marker has class word 2
- **THEN** it neither replaces the box nor raises the flag

## ADDED Requirements

### Requirement: Viewport attachment
A preset SHALL set or clear the viewport's wheel handler, add its editor or free-camera press and move handlers without removing any, and save the viewport's wheel handler first when the saved slot is empty. Detaching the controller SHALL unbind the camera node, remove the free camera's press and move handlers and restore the saved wheel handler; attaching SHALL only bind the node; setting the state it already has SHALL do nothing.

#### Scenario: A detach after a free preset
- **WHEN** a viewport with no wheel takes a preset with the wheel flag and the controller then detaches
- **THEN** the free press and move handlers are gone and the viewport has no wheel handler

### Requirement: Gamepad stick zoom
The stepped camera's stick zoom SHALL subtract `(steps · 0.008333334) · ((max − min) · 0.2)` from the zoom, take the least below it and otherwise the most above it (a NaN kept), and hold the steeper of the new zoom's pitch floor and the asked pitch.

#### Scenario: One notch in
- **WHEN** a zoom of 1280 in an 800–2000 range takes 120 steps with an asked pitch of 0.5
- **THEN** the zoom is 1040 and the held pitch is the floor 0.805

### Requirement: Cut-away focus
The cut-away driver SHALL centre its probe on the camera's stored point when it follows nothing, else on the followed object's own point when its node reports one, else on the object's position.

#### Scenario: A followed node's point
- **WHEN** the camera follows an object at (4, 5, 6) whose node reports the point (7, 8, 9)
- **THEN** the probe centres on (7, 8, 9)
