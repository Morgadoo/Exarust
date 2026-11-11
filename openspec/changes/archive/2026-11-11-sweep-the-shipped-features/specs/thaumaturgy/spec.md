## ADDED Requirements

### Requirement: The demon's size from its body
The summon's arrival SHALL take the demon's size for its shell from the demon's built body (its muscles' character stats), not from the spawn request the body build consumes.

#### Scenario: The demon arrives
- **WHEN** a summoned demon of size 1.5 arrives
- **THEN** its shell's texture scale is divided by 1.5

### Requirement: Debug devices report
`--debug-slot` SHALL log how many devices of the level took a crystal, also when the level has none, and the power tree's key SHALL log that it opened the tree.

#### Scenario: A level without devices
- **WHEN** `--debug-slot` runs in a level with no teleporter or reconstruction device
- **THEN** the log says that 0 crystals went on 0 devices
