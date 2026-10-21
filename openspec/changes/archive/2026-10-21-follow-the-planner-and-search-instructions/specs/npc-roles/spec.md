## ADDED Requirements

### Requirement: Corpse scripts
Handling a dead character SHALL give, for each of its roles in list order (invalid ones skipped), the corpse section of the role's first entry aimed at the Player with no condition, when that entry carries the corpse section; a role whose first such entry lacks it SHALL give nothing.

#### Scenario: First Player entry only
- **WHEN** a corpse holds a role with a corpse script for the Player, one with a corpse script for another role, and one whose first Player entry is an attitude entry followed by a corpse entry
- **THEN** only the first role's script is given

### Requirement: Interaction condition
The interaction condition of A toward B SHALL be the condition section of the entry the entry finder gives for that section.

#### Scenario: A condition entry
- **WHEN** A holds no role, then a role with a condition entry for the Player
- **THEN** there is no condition, then that entry's script

### Requirement: Roles script functions
The roles unit SHALL register `StartDialogue` and then `ChangeRole`; `ChangeRole` SHALL report that the function does not exist.

#### Scenario: Registration
- **WHEN** the registration is listed and `ChangeRole` is called
- **THEN** the names are `StartDialogue`, `ChangeRole`, and the error reads `Function "ChangeRole" does not exist.`

### Requirement: Level relation removal
Removing a relation role toward a level character while no level table is current SHALL select the locale's table first.

#### Scenario: No current table
- **WHEN** a relation role is removed toward a level character with no table current
- **THEN** nothing is removed and the locale's table is current
