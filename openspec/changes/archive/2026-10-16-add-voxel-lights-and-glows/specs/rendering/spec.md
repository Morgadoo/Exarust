## MODIFIED Requirements

### Requirement: Voxel light volume
The system SHALL build a level's voxel light volume as the original does. Each mesh's GI sample points mark a block of 20 cm voxels below the level's highest point. Every frame, light propagates through the open voxels from their eight diagonal neighbours, the level's base ambient, lights and voxel lights are injected, and the result is stored as 8-bit sRGB.

#### Scenario: Block height
- **WHEN** a level is taller than 12.8 m
- **THEN** its block is 256 × 256 × 128 voxels instead of 256 × 256 × 64

## ADDED Requirements

### Requirement: Voxel lights
The system SHALL add every voxel light (`0x3D0C` node) of a level and of its props' and placed objects' models into the GI volume each step, warm-up included, as the original does: lin(rgb)·√intensity·3·0.1 over 6 × 6 columns around the nearest voxel corner (outside the 8-column margins) and 7 slices, falling off as max(1 − 5000·d², 0)⁴, reaching about 1.2 voxels through geometry. Voxel lights never light a surface directly.

#### Scenario: Healing Salve on the menu shelf
- **WHEN** the main menu scene is shown at the original's camera after the GI warm-up
- **THEN** the wall, shelf and cobwebs around the Healing Salve on the left shelf are lit blue, as in the original's capture, where before only the jar glowed

#### Scenario: Reach
- **WHEN** a voxel is 3.62 voxels (72 cm) or more from a voxel light
- **THEN** that light adds nothing to it

### Requirement: Item glow by condition
The system SHALL scale an item's voxel lights directly under its model root by k = (1 − c/255)², c the condition byte (bits 16–23) of its placement flags, and the emissive colour of the meshes directly under that root by the same k, when the item's model section sets bit 0 (salves, power crystals), as the original does. Other items and level nodes are unchanged.

#### Scenario: Spent salve
- **WHEN** `exanima02` is shown, with a salve placed with condition byte `0xEE`
- **THEN** that salve's voxel light shines at 0.0044 of its intensity, about a fifteenth of its colour, and its glow mesh at 0.0044 of its emissive colour

### Requirement: Emissive surfaces
The system SHALL draw emissive as the original does: lin(colour), doubled for materials with lit or specular bits and an emissive layer, added on top and, with an emissive map on PBR materials, weighted by N·V²·z + 1 − z (z the fall-off's amount term, else 0). Unlit materials show only their emissive, with the opacity as alpha. Additive surfaces add colour × alpha over what is behind them. The GI takes nothing from emissive surfaces.

#### Scenario: Glow inside a glass jar
- **WHEN** the menu's Healing Salve is shown
- **THEN** its glass jar shows through its additive blue glow instead of being covered by an opaque blue shape

#### Scenario: Torch handle
- **WHEN** a torch is shown
- **THEN** its handle's emissive map glows at twice the record's emissive colour

### Requirement: Opacity fall-off
The system SHALL fade a surface whose record's fall-off amount exceeds 0.01 in magnitude with the view angle, as the original does: alpha += 2^((x − N·V)·y)·z, f = 2^(−1.8·exponent), a = (f + 1)·amount and y = 1.8·exponent, both negated with extra flag `0x100`, z = |a|, x = 1 when a < 0. The opacity loses (f + 1)·f; alpha is clamped to [0, 1] before blending. Unlit surfaces take it only with an emissive map.

#### Scenario: Salve glow outline
- **WHEN** the salve's glow (opacity 0, amount 1, exponent 2, flag `0x100`) is shown
- **THEN** its alpha is 1.08 where it faces the eye and 0.09 at its outline
