## ADDED Requirements

### Requirement: Cube pipeline cache
Each mesh and key SHALL reuse its specialized pipeline id from a cache instead of specializing again. A miss SHALL specialize once and store the id; a failure SHALL not be stored.

#### Scenario: First and later draws
- **WHEN** a mesh and key are prepared for the first time
- **THEN** the specializer runs once and the id is stored
- **WHEN** the same mesh and key are prepared again
- **THEN** the stored id is reused and the specializer does not run
- **WHEN** the specializer fails
- **THEN** nothing is stored
