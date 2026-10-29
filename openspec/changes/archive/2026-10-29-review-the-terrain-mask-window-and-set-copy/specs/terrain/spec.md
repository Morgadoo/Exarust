# Spec Delta

## ADDED Requirements

### Requirement: Terrain pass mask window by node level
Each compositor pass SHALL upload and sample a mask window of `256 >> L` texels holding `240 >> L` cells, starting at texel `(by·256 + bx)·30`, where `L` is the level of the node the pass draws: the block level 3 for the composite set passes, whatever the set's level, and the tile's root node (0) for the tile's mask and height passes.

#### Scenario: A low set's block
- **WHEN** block (7, 5) is baked into a level-4 set
- **THEN** its mask window starts at texel 38 610, is 32 texels wide and holds 30 cells

#### Scenario: The tile's height pass
- **WHEN** a tile's height pass is planned
- **THEN** it samples a 256-texel window out to 241/256 and its vertices reach 241/244 of a 244² viewport

### Requirement: Terrain set copy draws
A set copy SHALL draw one quad per destination target the features keep (colour always), sampling the source's same target at unit 0 with the vertex and texcoord at the same 0/1 corner.

#### Scenario: Normal off
- **WHEN** a set without the normal feature is copied
- **THEN** no normal-target draw is planned

### Requirement: Terrain extension deactivation order
The static terrain's deactivation SHALL run only while the terrain is active and SHALL deactivate its extensions in attach order.

#### Scenario: Two extensions
- **WHEN** a terrain with extensions attached as grass then a second one deactivates
- **THEN** grass deactivates first, and a second deactivation does nothing
