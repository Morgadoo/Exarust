# item-icons Specification

## Purpose
The item icons: how the off-screen camera frames an item, the pixel extents its slot keeps, how a reserved icon becomes an atlas cell, and how the icon scene stays out of the world.

## Requirements

### Requirement: Scene fit distance
The icon camera SHALL stand on +Z from the item box's centre at the engine's scene-fit distance: the first cross half extent unless it is below the second (then the second, with the depth weight halved), bent toward the framing distance F while F is above it by `F·((r·0.11 + 1) − 1/(8r + 1))` with `r = h/F`, divided by the runtime's `sin(fov/2)` with the icon view's 40° field, plus the depth half times its weight.

#### Scenario: A small item at the default framing
- **WHEN** the half extents are (5, 2, 1) cm and the framing is 70 cm
- **THEN** the distance is 77.0322 cm

#### Scenario: The second half wins
- **WHEN** the half extents are (2, 50, 10) cm and the framing is 30 cm
- **THEN** the distance is 151.1902 cm

### Requirement: Icon pixel extents
The plan's screen extents SHALL be the box's projected span in centimetres, widened by 0.0001, times `icon · 1/tan(20°) / distance · 0.5`.

#### Scenario: A metre box at the apparel framing
- **WHEN** a box of half extents 0.5 m is framed at 30 cm into a 160-pixel icon
- **THEN** each extent is 117.305 pixels

### Requirement: Pending icons become atlas cells
A panel's reserved icon SHALL be rendered off screen at most two a frame and its slot SHALL take an atlas cell and half extents of half its rendered size plus the GUI unit; a headless App SHALL paint the same scene on the CPU.

#### Scenario: Headless App
- **WHEN** a reserved icon has a baked scene and the App updates once without a renderer
- **THEN** the slot has a cell, the atlas holds pixels and the panel's icon layer draws one primitive

### Requirement: Off-screen isolation
The icon camera and sun SHALL carry the `Offscreen` marker on the off-screen layer, below every light's shadow layer, and the icon meshes SHALL cast no shadow, so no world system and no scene camera takes them.

#### Scenario: Spawned job
- **WHEN** an icon job is spawned
- **THEN** its camera and sun carry the marker and the off-screen layer, its parts are on that layer only and none is a shadow caster

### Requirement: Scene fit node
The engine's scene fit SHALL put the node it moves at the box's centre plus the direction from that node's old position to the view node, normalised by the engine's normalise, times the fit distance.

#### Scenario: A node at the view node
- **WHEN** the moved node sits where the view node is
- **THEN** it lands on the box's centre
