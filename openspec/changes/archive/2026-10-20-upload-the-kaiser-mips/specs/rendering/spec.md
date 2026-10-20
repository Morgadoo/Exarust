## ADDED Requirements

### Requirement: Kaiser texture mips
An RGBA8 texture with one stored level, power-of-two dimensions and both sides at least 4 SHALL get the Kaiser chain appended, and its mip level count SHALL become the base plus the generated levels. Every other texture SHALL keep its stored levels.

#### Scenario: One-level and stored-level textures
- **WHEN** a 4×4 RGBA8 texture stores only its base level
- **THEN** a 2×2 level is appended and the count becomes 2
- **WHEN** a texture stores two levels or has an odd size
- **THEN** no level is generated
