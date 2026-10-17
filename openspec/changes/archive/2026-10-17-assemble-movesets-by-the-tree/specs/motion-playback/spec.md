## MODIFIED Requirements

### Requirement: Moveset fallback
The system SHALL link the movesets into a tree, each one's parent being the nearest moveset before it named exactly as its parent, and SHALL build a character's clip table from its base moveset (the first of that name, case ignored) and the base's ancestors, overlaid from the root down: each clip a moveset defines replaces the table's. It SHALL build each table once per base and held-item config; a base no moveset is named after SHALL play nothing.

#### Scenario: Inherited walk
- **WHEN** the `1H & Shield` moveset's walk is requested
- **THEN** the walk of its ancestor `Default` is returned

#### Scenario: Names used twice
- **WHEN** the movesets of the installed `humanoid.rsq` are linked
- **THEN** the golem's `1H & Shield` (#16) hangs under the golem's `One Handed` (#15), not the human's (#1), and the ogre's under the ogre's

## ADDED Requirements

### Requirement: Clip table from held items
The system SHALL overlay on a character's table the moveset its held-item config names (low 16 bits: One Handed 0x1 … Polearm 0x40), searched under each ancestor of the base from the root down, children first, each one found with its ancestors below that ancestor; then the Torch (0x10010000) and Flaming (0x10020000) overlays when the config shares a bit with their mask, searched among the children of the weapon movesets found and of their ancestors below that ancestor.

#### Scenario: Golem with a sword
- **WHEN** a golem's table (base `Golem`) is built for the One Handed config
- **THEN** it draws on `Default`, `Golem`, the human `One Handed` and the golem's own `One Handed`, whose clips win

#### Scenario: Torch over a shield
- **WHEN** a human's table is built for a shield with a light in the main hand
- **THEN** Walk, Cmbt - Idle, Attack Mid and Idle come from `Torch` and the Defence stance from `1H & Shield`

#### Scenario: Human weapons
- **WHEN** a human's table is built for one held weapon or a light
- **THEN** it holds the same clips as the chosen moveset's own chain of parents
