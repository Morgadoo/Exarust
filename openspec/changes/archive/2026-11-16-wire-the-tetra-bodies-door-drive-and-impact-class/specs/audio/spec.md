## ADDED Requirements

### Requirement: Rigid body impact class
When a plain rigid body registers, each of its parts whose collision sound set has no shipped variant SHALL lose its impact class, so it makes no level impact and a held non-striking item makes no contact sound; a part with a sounded set SHALL keep it. With no game loaded the parts SHALL be left as built.

#### Scenario: Wood and stone
- **WHEN** a body whose set `wodcol` has 4 variants and one whose set `stncol` has none register
- **THEN** the first keeps its impact class and the second loses it

### Requirement: Door leaf slam fallback
A door leaf whose own collision set has no shipped variant SHALL ring its record's slam set for its impacts and keep its impact class through it; only the leaf's own part SHALL take that set.

#### Scenario: A stone leaf
- **WHEN** a door leaf whose set `stncol` has no variant and whose slam set `doora_slm` has 2 registers
- **THEN** it keeps its impact class and its impacts ring `doora_slm`
