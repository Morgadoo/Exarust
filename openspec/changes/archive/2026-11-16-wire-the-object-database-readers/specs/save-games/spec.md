## ADDED Requirements

### Requirement: Older world databases on a load
A save's `#worldobjdb` of an older layout SHALL go through the loader's upgrade chain on a load: `0xDBAC0D08` converts the operative section, `0xDBAC0D07` the weapon and then the operative section, and `0xDBAC0D06` the bodies and the random tables' condition words first. The oldest bodies' paint seeds SHALL be drawn from a run-time library twister seeded from the clock.

#### Scenario: Three older layouts
- **WHEN** story saves whose world database has the magic `0xDBAC0D08`, `0xDBAC0D07` and `0xDBAC0D06` are loaded
- **THEN** the live world tier holds the converted operative in each, the converted weapon from `0D07` on, and the converted body and condition word only from `0D06`

### Requirement: Class change on a load
A saved world instance whose header's material class differs from its template's SHALL take its appearance layers from the template on the load's migration; one of the same class SHALL keep its own.

#### Scenario: Two saved instances
- **WHEN** a save holds two instances of one template, one of another class
- **THEN** after the load only that one's first layer is the template's, its material the set's first
