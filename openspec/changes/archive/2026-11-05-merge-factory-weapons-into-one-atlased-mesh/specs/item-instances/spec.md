# Spec Delta

## ADDED Requirements

### Requirement: Factory weapon mesh
A factory weapon SHALL be one mesh: every chosen part whose component loads is appended, in recipe order, to one new mesh, which is then regrouped by material, welded and converted to render form with its normals rebuilt from the assembled shape. Every group SHALL draw with the weapon's one material. This SHALL hold for every factory in `Factories.rpk`.

#### Scenario: A golem halberd
- **WHEN** `@GlmHalbrd A` is assembled
- **THEN** its four parts are one node with one mesh drawn with one material

### Requirement: Factory part placement
An appended part SHALL be its component's first mesh node after the part's deforms, its first texture-coordinate stream mapped to `uv · size + offset` (the part's atlas rectangle, multiplied then added in single precision), placed by the part's frame: the turn of its angles with each row multiplied component-wise by the part's scale, and its placed position.

#### Scenario: A scaled, turned part
- **WHEN** a part turned a quarter about Y has scale (2, 1, 1)
- **THEN** its component's (0, 0, 1) is placed 2 units along the weapon's X, the scale acting after the turn

#### Scenario: Atlas coordinates
- **WHEN** a part's rectangle is offset (0.25, 0.5) with size (0.5, 0.25)
- **THEN** its texture coordinate (0.5, 1) becomes (0.5, 0.75)

### Requirement: Factory weapon atlas
A factory weapon's material SHALL read one colour, normal and surface target at the factory's atlas size (halved below texture detail 3), cleared to black, a flat normal and (0, 0.01, 0). Every part's region SHALL be drawn over the atlas texels its rectangle covers, mirrored for a negative size, blending into what earlier regions drew: every region's colour and normal passes first, then every region's specular pass over the finished normals.

#### Scenario: Overlapping regions
- **WHEN** a sword's guard region spans the blade's columns
- **THEN** the blade's texels the guard's masks leave clear keep the blade's paint

### Requirement: Factory striking extents
The striking extents a factory weapon writes SHALL be the head's (or the flipped side's) stored box bottom and top over its placed position, as 1/200ths of the merged mesh's box top, rounded; an end past the merged top SHALL be stored above 200.

#### Scenario: A tool's head past the merged top
- **WHEN** an axe's head box reaches 0.5% above the merged mesh's top
- **THEN** its end byte is 201
