## MODIFIED Requirements

### Requirement: Kaiser texture mips
An uncompressed texture with one stored level whose word asks for mipmapping SHALL get the Kaiser chain from its first uploaded level, as far as its dimensions halve exactly and their sum stays at least 3, and its mip level count SHALL be the levels it then holds. A compressed texture with one stored level SHALL get no chain, also when it is decoded on the CPU. Every other texture SHALL keep its stored levels.

#### Scenario: One-level and stored-level textures
- **WHEN** a 4×4 RGBA8 texture stores only its base level
- **THEN** a 2×2 level is appended and the count becomes 2
- **WHEN** a texture stores two levels, or is a single-level BC4 image decoded on the CPU
- **THEN** no level is generated
