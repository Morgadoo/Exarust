# world-viewer Specification

## Purpose
Shows the original game's models and levels inside the Bevy runtime, as the
first step from decoded data to a playable world.

## Requirements

### Requirement: Model view
`exarust model <pack> <entry>` SHALL display every static mesh of the entry with its textures, framed by a camera that can fly around it.

#### Scenario: Prop
- **WHEN** `exarust model Objlib afr_cornflag_a01` runs
- **THEN** the textured plant is visible at its real size with alpha-masked leaves

### Requirement: Level view
`exarust level <name>` SHALL display the level's own scene meshes and every prop placed from the object library at its stored transform, with lighting and a free-fly camera.

#### Scenario: Story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's props appear in their stored arrangement and the camera can move through the level

### Requirement: Shared resources
The viewer SHALL load each distinct model, material and texture once, however many times it is placed.

#### Scenario: Repeated props
- **WHEN** a level places the same object hundreds of times
- **THEN** its meshes and textures are decoded and uploaded once

### Requirement: Missing data is not fatal
The viewer SHALL show what it can when a prop's object, a material or a texture is missing, and log what was skipped.

#### Scenario: Unknown object
- **WHEN** a prop names an object absent from the installation
- **THEN** the level still loads and the missing name is logged once

### Requirement: Responsive load
The viewer SHALL build a level over frames, and SHALL run on the async compute pool the work that cannot be cut into short pieces, so the window keeps answering the desktop while a level loads.

#### Scenario: Large level
- **WHEN** `exarust play exanima01` loads
- **THEN** no frame of the load takes a second, and the desktop never reports the window as not responding

### Requirement: Complete first frame
Level and play modes SHALL end their load only when the voxel GI block around the starting view is built and warmed up and, in play mode, when the level's collision is built and every character is dressed. With GI turned off, the load SHALL end without waiting for it.

#### Scenario: First frame after the load
- **WHEN** `exarust play exanima01 --screenshot first.png --frames 1` runs
- **THEN** the screenshot shows the level lit by its GI and the player in braies, not the loading screen

#### Scenario: GI off
- **WHEN** `exarust play arenao1 --no-gi` runs
- **THEN** the load ends once the characters are dressed

### Requirement: Scripted runs after the load
`--frames` (before a screenshot) and `--shove` SHALL count frames from the end of the load, and `--autowalk` SHALL start walking then.

#### Scenario: Screenshot
- **WHEN** `exarust play exanima01 --screenshot shot.png --frames 60` runs
- **THEN** the screenshot shows the level 60 frames after the loading screen went

### Requirement: Window identity
The window SHALL carry the app id (X11 class) `exarust`, so the desktop names it in its dialogs.

#### Scenario: Wayland
- **WHEN** the window opens on a Wayland desktop
- **THEN** its toplevel's app id is `exarust`

### Requirement: Character creation scene
`exarust character` SHALL show the body in the original's character-creation scene: the `sgccflr01` floor and its two omni lights, with no sun and no ambient. Every surface SHALL fade to black by 2 m horizontal distance from the origin, over a black background. The camera SHALL be the original's: target (0, 0.6, 0) m, 4 m along (−2, 2.1, 3.6), 50° vertical field of view. A "Torch" or "1H & Torch" action SHALL show the body holding the lit torch in the hand that moveset raises.

#### Scenario: Default view
- **WHEN** `exarust character Default/Idle` runs
- **THEN** the body stands on cobblestones in a pool of white and warm light with black all around

#### Scenario: Torch moveset
- **WHEN** `exarust character Torch/Idle` runs
- **THEN** the body holds the burning torch raised in its right hand

#### Scenario: Torch beside a weapon
- **WHEN** `exarust character "1H & Torch/Idle"` runs
- **THEN** the body holds the burning torch raised in its left hand
