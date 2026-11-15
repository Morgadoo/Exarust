## ADDED Requirements

### Requirement: Teleports hold the body
A teleport SHALL hold the body around its placement: its world contacts go and the kept pairs with it are dropped, then its held items are put back. The Long Door's move and Vanish's come-back SHALL teleport.

#### Scenario: A Long Door moves the caster
- **WHEN** a Long Door moves its caster
- **THEN** the caster's kept pairs are dropped and its world contacts are empty

### Requirement: Vanish comes back where it stands
Vanish's come-back SHALL teleport the body to its own standing point, so the body keeps its place.

#### Scenario: A vanished body comes back
- **WHEN** a let-go Vanish reaches its come-back
- **THEN** the body is teleported where it stands and its held items are put back

### Requirement: Spirit body is disabled
Spirit Form's new body SHALL be built disabled down its parts: out of the body-to-body pairs, still on the level.

#### Scenario: The spirit body is built
- **WHEN** the spirit body's simulated body is built
- **THEN** it is disabled and settles for 16 ticks
