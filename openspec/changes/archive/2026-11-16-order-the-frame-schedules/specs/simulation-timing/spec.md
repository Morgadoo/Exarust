## MODIFIED Requirements

### Requirement: The game's schedules leave no conflicting pair unordered
The workspace tests SHALL build the game's App as the binary builds it, headless in lockstep, and fail on any two systems of a fixed-step schedule, of the character tick, of the brains' chain, of `PreUpdate`, `Update` or `PostUpdate` that touch the same data with no order between them, naming both systems and the data.

#### Scenario: A plugin adds a system that writes the bodies
- **WHEN** a lane adds a fixed-step system that writes `Ragdoll` with no order against the systems that read it
- **THEN** the check fails and names each such pair and `Ragdoll`

#### Scenario: A plugin adds a frame system
- **WHEN** a lane adds an `Update` system that writes the GUI's frames with no order against the other GUI draws of its phase
- **THEN** the check fails and names each such pair

## REMOVED Requirements

### Requirement: The frame's Update and PostUpdate keep a TODO count
**Reason**: `Update` and `PostUpdate` are ordered; the check holds them to no unordered pair like the other schedules.
**Migration**: A new frame system joins its phase and orders itself against the systems of the phase it conflicts with.

## ADDED Requirements

### Requirement: The frame's Update runs in phases
Every game system of `Update` SHALL sit in one of the chained phases `Input`, `Gui`, `Hub`, `Session`, `Controls`, `Camera`, `Hand`, `Cursor`, `World`, `Audio`, `Draw`, `Capture`, after the original's frame order: the GUI's input and windows, the session's frame, the player controls' frame, the camera, the held item and the use list, the cursor, the world's draw preparation, the sounds, the HUD's draw, then the captures.

#### Scenario: The GUI before the session
- **WHEN** a frame runs a GUI click that opens a container and the flow's steps
- **THEN** the click's systems run before every system of the session's frame

### Requirement: The mind's hand-back runs between the session and the controls
The mind's hand-back transition, an exclusive system, SHALL run after every system of the `Session` phase and before every system of the `Controls` phase.

#### Scenario: A spirit returns home
- **WHEN** the hand-back's black screen calls its action in a frame
- **THEN** the session's frame of that frame has run and the player's controls run after the swap

### Requirement: The frame's PostUpdate runs in phases placed against Bevy's sets
Every game system of `PostUpdate` SHALL sit in one of the chained phases `Prepare` (before the transform propagation, the camera update, the UI and the asset events), `Placed` (after the propagation, before the bounds, the frusta, the visibility, the light bounds and the sun's cascades), `View` and `Trace`.

#### Scenario: An effect writes a light's transform
- **WHEN** the arcs place their voxel lights after the transforms propagate
- **THEN** the visibility and the light frusta of that frame read the placed lights

### Requirement: Allowed frame pairs are only with Bevy's unnamed systems
The allowlist SHALL hold, for `Update` and `PostUpdate`, only pairs with one of Bevy's own systems that no public set or path names, and an entry MAY name a module to cover its systems.

#### Scenario: A gizmo system
- **WHEN** the arcs' draw and Bevy's private light gizmos touch global transforms
- **THEN** the pair passes through the entry that names the gizmo module and its reason
