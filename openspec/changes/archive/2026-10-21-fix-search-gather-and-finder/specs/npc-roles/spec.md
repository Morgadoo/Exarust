## MODIFIED Requirements

### Requirement: Entry finder
The entry finder SHALL take, for each of A's roles newest first, the role's first entry aimed at one of B's roles, then at one of A's relation roles toward B, and SHALL give the requested section of the first such entry that carries it, read through the section table, so the corpse section SHALL never be given.

#### Scenario: An attitude entry hides a condition
- **WHEN** one role's only entry for the Player is a condition, and another role's first entry for the Player is an attitude followed by a condition
- **THEN** the first gives its condition and the second gives none

#### Scenario: No corpse section
- **WHEN** a role's entry for the Player holds a corpse script and the finder is asked for the corpse section
- **THEN** it gives none, while the entry's corpse accessor finds the script
