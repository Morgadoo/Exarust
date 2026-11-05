## ADDED Requirements

### Requirement: Ground Query floor pieces
The feet routine's Ground Query SHALL meet only the colliders built from a floor node's meshes: a node whose second flag word has `0x800000`, or a tile grid cell's floor parts (the original's merged `+Floor` node gains that bit). Other level geometry SHALL be passed through.

#### Scenario: A plain mesh over a floor mesh
- **WHEN** a plain flat mesh lies 20 cm over a floor-flagged one and the query looks down from 30 cm
- **THEN** it returns the floor-flagged mesh's height

#### Scenario: No records
- **WHEN** the level has no floor records
- **THEN** the query misses

### Requirement: Sole floor compare
The sole floor SHALL take the query's hit only when the low floor (the ground, 10 cm less while touching, plus the leg's offset) is under it; an unordered compare SHALL keep the low floor.

#### Scenario: NaN hit
- **WHEN** the hit is NaN
- **THEN** the sole floor is the low floor

### Requirement: Grip mode writes
A held item's grip mode SHALL change only when its holder starts a motion (the clip's rule) or the holder's locomotion routine runs (1 running or in combat, else 2). On a tick with both, the locomotion's mode SHALL win over a behaviour's start, and the walk the locomotion starts SHALL win over the locomotion's mode.

#### Scenario: Running with a two-hander
- **WHEN** the holder's walk clip plays and its locomotion runs with the run leg mode
- **THEN** the mode is 1, not the walk clip's 2

#### Scenario: Walk start
- **WHEN** the locomotion starts the walk on a tick it writes 1
- **THEN** the mode is 2
