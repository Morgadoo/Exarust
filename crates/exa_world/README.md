# `exa_world`

Levels in the Bevy world.

**Phase:** 2. Status: props, level scene nodes and lights spawn
(`spawn_level`, `spawn_model`); walls and floors from the tile grid, terrain
and the grid's rooms; portal clipping is next (see
[`docs/PLAN.md`](../../docs/PLAN.md)).

- `spawn_model(commands, model, placement)`: a cached model with its node hierarchy, its lights, fires and voxel lights (`exa_render::gi::VoxelLight`) and its nodes' keyframe tracks (`exa_anim::tracks::NodeTracks`); a placement replaces the root transform, as level props do. Placed items dim their voxel lights and glow meshes by their condition byte. `spawn_model_nodes` also returns the node entities.
- `bodies`: which rigid bodies of a level register in the solver, as the original registers them (each body tree from its top body down through its body children), and their registration: the free trees of props not placed static (`Prop::STATIC`; nodes marked static, `rfc::Node::STATIC`, never register), of placed objects that are no door, lever or push surface, and of the level's own model, asleep; trees hung from the world and mechanisms stay static until their changes land (`LevelInfo::bodies` counts them).
- `spawn_level(commands, loader, name)`: the level's own meshes and omni lights plus every prop from `Objlib.rpk`.
- `LevelLoad`: the same level built a step at a time within a time budget per step (grid, terrain, level model, props, placed objects, lights), for a loading screen.
- `PlacedCharacter`: a level's placed character resolved for spawning (position, facing, dead or alive, race, worn and held items) with its whole record, from which play mode derives its stats, and the encumbrance its worn items and held shield give it (the level's or the game's items).
- `rooms`: the grid's rooms and door links (the original's flood fill) and `RoomTracker`, the original's active, adjacent and hidden rooms around an anchor with their 1 s fades.
- `room_view`: the rooms in the Bevy world: `LevelRooms`, `RoomAnchor` (the player, or 20 m ahead of the fly camera), `InRoom` on meshes, rooms of props and characters, and the rooms' black shells (`RoomShell`: occluder, lid, fade overlay) shown by state; `RoomView { all }` shows every room.
- `grid`: walls and floors from the wall and floor sets, and each room's occluder and lid merged from the pieces' `[…` and `+R…` strips and the ceiling slots (never spawned as pieces); ceiling caps.
- `terrain`: tiles, blend rules, height pass and plants; in a grid level only the 6 m blocks the grid flags, one mesh per block room.
- `gi_volume`: the voxel GI occupancy block, built on the async compute pool around the camera's focus; each floor piece's points come from the original's floor heightfield (`exa_physics::voxel`), and `GiState` tells a loading screen where it stands.
- `point_light(light)`: the original omni light approximated as a Bevy `PointLight`.
- `nav`: the original's navigation grid (30 cm cells, two layers, cost bytes), built for each level when its load ends from its terrain (the 6 m blocks the level builds), its meshes' GI sample points and its tile-grid floors, on the async compute pool (`NavGrid` resource, `NavState`); `--debug-nav`/F6 draws it, `nav::headless` builds it without a window ([`docs/NAVIGATION.md`](../../docs/NAVIGATION.md)).
