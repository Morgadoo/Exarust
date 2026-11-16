## MODIFIED Requirements

### Requirement: Light alias spread
The system SHALL find a light's alias in a linked sector or make one, a ranged light only where its sphere reaches the sector's box; an alias with the light's stamp SHALL prune the walk, an older one SHALL be refreshed and marked moved, and the walk SHALL recurse through each sector that returned an alias, skipping busy sectors.

#### Scenario: A ring of rooms
- **WHEN** a light spreads from room 1 of rooms 1, 2 and 3 linked in a ring
- **THEN** rooms 2 and 3 each get one alias and no room stays busy

## ADDED Requirements

### Requirement: Alias link method choice
An alias of a ranged light SHALL link a node by the gated reach test, asking the cell flood's wall test as its gate when the scene is in grid mode; an alias of an unranged light SHALL link it by the plain luma test. Both SHALL weigh against the node's colour bound, and an alias no sync has written SHALL miss.

#### Scenario: Walls between a light and a mesh
- **WHEN** the wall test refuses a ranged light for a mesh in a grid scene
- **THEN** the mesh's bucket for that light weighs 0 with no faces, and an unranged light's bucket keeps its luma weight

### Requirement: Alias shadow word writes
A gated test past its intensity gate SHALL merge the new and old face masks into the alias's shadow word for a node with the casting bit; a gated miss SHALL leave the word; a lit plain test SHALL set it to 1. The plain test SHALL answer lit when unlit, as the original's stray return byte does.

#### Scenario: A miss after a hit
- **WHEN** a gated miss follows for a casting mesh whose bucket held faces
- **THEN** the alias's shadow word is unchanged

### Requirement: Activation alias sync
After a light's activation and flood, every alias the light owns SHALL take the light's link values at the light's frame in the sector (colour, its luma, falloff, radius, spot shaping), the light's node directly under its sector, and SHALL be marked moved.

#### Scenario: A registered light
- **WHEN** a ranged light at (3, 1, 1) m activates in a sector
- **THEN** its alias there carries its link values at (3, 1, 1) and is moved

### Requirement: Live link values
In the live scene every light node SHALL keep its link values: the linear colour times the current intensity, its luma, the 0.01 cutoff, the falloff 0.8/R and radius R of a ranged light (none and 3.4e38 unranged) and the record's spot shaping; the ranged bit SHALL be set on its option word.

#### Scenario: An unranged light
- **WHEN** a light whose record is unranged registers
- **THEN** its node lacks the ranged bit and its aliases run the plain test

### Requirement: Live mesh colour bound
In the live scene every mesh node SHALL take its material's linear specular times level plus diffuse as its colour bound, and a bound whose squared length exceeds the engine's epsilon SHALL ask for light buckets.

#### Scenario: A lit room mesh
- **WHEN** a room mesh with diffuse (0.4, 0.3, 0.2) and specular (0.1, 0.1, 0.1) registers with a ranged and an unranged light in its room
- **THEN** its bound is (0.5, 0.4, 0.3), each bucket carries its alias's weight and the heavier leads the chain

### Requirement: Alias link comparison switch
`--no-alias-links` SHALL turn off the alias sync, the colour bounds and the link methods of the live scene, leaving the stand-in test that links nothing.

#### Scenario: The switch off
- **WHEN** a level runs with `--no-alias-links`
- **THEN** no mesh bucket carries a weight
