# Spec Delta

## ADDED Requirements

### Requirement: Cut-away candidates
A mesh whose node's second flag word has `0x20000000` SHALL be a cut-away candidate of kind `flags2 & 0x70000000`; a mark SHALL carry to its parent under `0x01000000` unless the parent is not a mesh, and to its mesh children under `0x02000000`.

#### Scenario: The shipped candidates
- **WHEN** every Objlib and Resource chunk tree is read
- **THEN** 142 Objlib and 38 Resource nodes are candidates (Objlib kinds `0x2…` 94, `0x3…` 11, `0x6…` 37) and none fades all the way

### Requirement: Cut-away draw
A fading mesh SHALL be drawn after the blended surfaces, its nearest surface blended over the scene at an opacity of one less its fade amount in place of its diffuse alpha. A settled mesh SHALL be drawn the same way without supersampling, and opaque into half of each pixel's supersampled sub-frames with it. A mesh faded all the way SHALL not be drawn. The blended surfaces of a fading or settled mesh SHALL not be drawn. The shadow passes SHALL ignore the fade.

#### Scenario: A roof over the player
- **WHEN** the player stands under exanima02's scaffold and the camera looks down on them
- **THEN** the planks over the player are drawn at half opacity and the player shows through, while the planks' shadows stay

#### Scenario: Turned off
- **WHEN** `EXA_CUT_AWAY=0` is set
- **THEN** every candidate is drawn solid
