## ADDED Requirements

### Requirement: Force effect programs
The force ribbon and force-shell programs SHALL reproduce the shipped `effectforce_f`, `effecttorrent_f` and `effectshell_f` arithmetic in WGSL and clamp their output to [0, 1]. The shell's copy SHALL move out 2 cm along its skinned normal. Each program SHALL compose and validate with naga, and each uniform struct SHALL match its WGSL layout in size and offsets.

#### Scenario: The programs validate
- **WHEN** the ribbon program is composed for the blast and the torrent, and the shell program for skinned and rigid meshes
- **THEN** naga validates every variant

#### Scenario: The shell uniform layout
- **WHEN** the shell uniform is laid out
- **THEN** it is 256 bytes with `ColPt` at 96 and `ColourInt` at 240, in Rust and in WGSL

### Requirement: Force effect render state
The force programs SHALL blend as the materials' records' blend word 0x163 (source ONE, destination ONE_MINUS_SRC_ALPHA) without depth writes, drawing the force record two-sided and the aura record one-sided.

#### Scenario: The blend word
- **WHEN** the blend word 0x163 is mapped through the original's factor table
- **THEN** it is Bevy's premultiplied blend

### Requirement: Force noise texture
The `_ForceNoise` texture SHALL be 256 by 256 one-byte red texels, repeating and linearly filtered, with its 2×2 box mip chain when the parameter word's bit 0x40 is set.

#### Scenario: The mip chain
- **WHEN** the picture is uploaded with the full parameter word
- **THEN** it holds nine levels, 87 381 bytes
