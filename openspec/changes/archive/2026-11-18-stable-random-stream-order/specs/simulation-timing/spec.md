## ADDED Requirements

### Requirement: Particle streams keyed by spawn order
A fire's or a portal's particle system SHALL seed its random streams (and a fire its ring's source places) from the next number of its world's flame count, taken when its spawn applies, mixed with the fire node's loaded place, and SHALL NOT key them by an entity index, which a build that adds an unrelated resource or message type moves.

#### Scenario: Two builds, one more resource
- **WHEN** the same fire is spawned in two worlds, the second holding 37 more entities before it
- **THEN** both fires' flames and beds are identical, and a fire at another place differs

### Requirement: Shared generators walked in entity order
A system that draws one shared generator once per entity (the light flicker, the footsteps, the body sounds, the collapse groans, the level impacts, the thaumaturgy effects' ticks and contacts, the golem alarm, the item abilities) SHALL walk its query in entity (spawn) order, and the scene SHALL register new lights and meshes and give lights their ids in that order, not in the query's table order.

#### Scenario: Two lights in two tables
- **WHEN** two flickering lights sit in different tables, the later-spawned one in the table the query walks first
- **THEN** the earlier-spawned light still takes the generator's first draw
