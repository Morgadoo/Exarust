## ADDED Requirements

### Requirement: Unranged lights do not fall off
A point light without its range flag, or with no radius, SHALL light every surface at full strength, with no distance falloff, as the original leaves its attenuation at 0; a ranged light SHALL fall off as max(1 − 0.8·d/R, 0)⁴.

#### Scenario: An unranged light far away
- **WHEN** a surface lies 100 m from an unranged light and 5 m from a ranged light of radius 8 m
- **THEN** the unranged light's falloff is 1 and the ranged light's is 0.5⁴

### Requirement: Front-culled cube casters
A mesh whose render flags carry 0x800 SHALL be drawn into its lights' cube shadow faces with its front faces culled; every other opaque caster SHALL keep the back-face cull.

#### Scenario: Two casters in one cube
- **WHEN** a light's six faces draw a plain caster and one with render flag 0x800
- **THEN** the flagged caster's six draws cull front faces and the plain caster's cull back faces
