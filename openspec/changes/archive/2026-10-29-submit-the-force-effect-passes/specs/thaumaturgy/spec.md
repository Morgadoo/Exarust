## ADDED Requirements

### Requirement: Force ribbons reach the GPU
The running game SHALL draw the force blast, bolt and wave ribbons with `effectforce_f` and the torrent's streams with `effecttorrent_f`, binding each frame's `TexOfs` from the effect's frame and the `_ForceNoise` picture as `EmiTex`.

#### Scenario: A blast is thrown
- **WHEN** a thrown blast emits its ribbon
- **THEN** the ribbon wears the blast program with the frame's `TexOfs` (0, radius, texture scale)

### Requirement: Force-shell aura reaches the GPU
A Shell cast's aura SHALL be drawn with `effectshell_v`/`effectshell_f` over copies of the caster's skinned meshes, its uniforms taken each frame from the shell's CPU uniforms for a skinned copy. An aura's copies SHALL leave the scene with the aura.

#### Scenario: A Shell cast draws its aura
- **WHEN** the player casts the Shell power
- **THEN** the aura node copies each skinned mesh of the player's body with one shell material whose fade is 1 − shown

#### Scenario: The aura closes
- **WHEN** the aura has closed and left the scene
- **THEN** its copies are removed

### Requirement: Force noise only while needed
The `_ForceNoise` picture SHALL be made the first frame a force effect is alive and uploaded again only when the noise object reports a new picture; with no force effect alive no noise work SHALL run.

#### Scenario: No power active
- **WHEN** no shell aura, blast-family effect or torrent exists
- **THEN** the noise picture is neither made nor uploaded
