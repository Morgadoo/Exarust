## ADDED Requirements

### Requirement: In-process level change
A level change SHALL despawn every level-owned root with its subtree, leave the physics world with no body or collider of the old level, drop the level's GI, navigation, room, wall, terrain-ground, sun-map, sky and fog state, and keep the player entity and the session state; the destination SHALL load through the same staged level load as a start-up load. A teardown step in the same batch as a load step SHALL make one change, and a placement step SHALL be carried across the load in metres.

#### Scenario: Teardown leaves nothing of the level
- **WHEN** a level root with a child, a placed character, a player holding a capsule handle and one physics body with a collider are present and a teardown is requested
- **THEN** no level entity or child remains, the player remains without its capsule handle, and the physics world holds 0 bodies and 0 colliders

#### Scenario: The player survives the load
- **WHEN** a running `exanima01` changes to `arenahub`, then `arena1`, then back to `exanima01`
- **THEN** each level holds one player, the physics world holds only that level's bodies and colliders, and `exanima01` holds the same counts on its return as on its first load within a few entities

#### Scenario: Load, teardown and placement in one batch
- **WHEN** the steps teardown, load `arenahub` at spawn 2 and place the player at (100, 200, 300) cm facing 90 degrees are applied
- **THEN** one change to `arenahub` at spawn 2 is requested and the player placement is (1, 2, 3) m with a yaw of pi/2

### Requirement: Practice next fight reload
The practice ladder's next fight SHALL reload its arena (`arena`, or `arenae` for the expert ladder) and run the practice scene filter right after the load, before the heal and the placement.

#### Scenario: Expert next fight
- **WHEN** the expert ladder's fight 7 advances
- **THEN** the steps load `arenae` with a teardown and the next step is the practice scene filter
