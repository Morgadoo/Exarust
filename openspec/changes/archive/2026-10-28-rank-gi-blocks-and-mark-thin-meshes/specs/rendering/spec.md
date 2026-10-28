# Spec Delta

## ADDED Requirements

### Requirement: GI block residency
The system SHALL rank the level's built GI blocks every frame by max(|f − c| − 2400, 0) per ground axis in centimetres from the focus (NaN not clamped), squared and summed, with a stable insertion sort that moves a block forward only past a strictly farther one, SHALL build next the first unbuilt or dirty block in that order, and SHALL keep built occupancies for the level.

#### Scenario: Unbuilt blocks
- **WHEN** only the first of four blocks is built and the focus is 100 m from it
- **THEN** the order is blocks 1, 2, 3, 0 and block 1 is built next

### Requirement: GI light texture handoff
The system SHALL give the light textures to the first nine blocks in creation order, and each frame SHALL give a built block among the first nine in rank order that has no texture the texture of the farthest textured block beyond them.

#### Scenario: Focus at the far end of a row
- **WHEN** twelve built blocks lie in a row 48 m apart and the focus is at the last one
- **THEN** the last three blocks take textures 0, 1 and 2 from the first three, and the nine nearest blocks step

### Requirement: GI thin meshes
The system SHALL weigh a GI mesh's points a quarter when its first surface's material has render class bit 0x4000, or has an opacity layer and an opacity above 0.75 (an ordered compare).

#### Scenario: Cut-out material
- **WHEN** a mesh's first material has an opacity layer and opacity 0.76
- **THEN** the mesh is thin, and with opacity 0.75 or NaN it is not

### Requirement: GI top slice
The system SHALL draw a sector group's light step from its floor to the block's top slice inclusive when the environment record has flag 0x20, and otherwise SHALL clear the top slice every step.

#### Scenario: Outdoor terrain arena
- **WHEN** a group with floor 3 steps in a 64-slice block of a level whose record has flag 0x20
- **THEN** it draws slices 3 to 63, and slices 3 to 62 without the flag
