## REMOVED Requirements

### Requirement: Props meet only the level
**Reason**: Bodies collide with each other through the pairs; a prop body meets characters, corpses and other props.
**Migration**: See Props meet bodies; the stand-ins stay for the character capsules only.

## ADDED Requirements

### Requirement: Props meet bodies
The system SHALL test a prop body's spheres against the level and meet other bodies' parts (characters', corpses', other props') through the contacts between bodies, by its mobility, the group masks of its tree (a child never meets its parent) and no layer. The meshes a body moves SHALL stay in the physics world as its stand-ins, moved with it, for the character capsules; no body's spheres SHALL meet a stand-in.

#### Scenario: A cup on a table body
- **WHEN** a cup body (mobility 0.5) drops onto a sleeping table body (mobility 0.05) standing on the floor
- **THEN** it wakes the table, comes to rest on its top, and both sleep again with the table where it stood

#### Scenario: Pushed by a limb
- **WHEN** a character's limb thrown at 2 m/s meets a prop resting on the floor
- **THEN** the prop wakes and is pushed away

#### Scenario: Over a stand-in
- **WHEN** a prop body and a character's limb are woken over another body's stand-in lying 1 m over the ground
- **THEN** both fall through the stand-in to the ground

#### Scenario: Stand-in
- **WHEN** a body registered at the origin moves 1 m along x
- **THEN** its stand-ins move 1 m along x
