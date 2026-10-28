# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the game flow, save bridge
and arena paragraphs: Exarust could load a level only at start-up. The hub's
match launch and return restarted the process with the career in a
hand-off file, and the flow's level loads (story start and respawn, practice
start and next fight, level exits and teleporters) were logged and left the
loaded level in place.

## What Changes

- `exa_world::level_lifecycle`: every root a level owns carries
  `LevelEntity` (level root, placed characters, scene bodies, the debug
  duellist, room shells, sky). A `LevelChangeRequest` despawns those
  subtrees, empties the physics world (`Physics::clear`, new capsules for
  the surviving characters) and drops the level's GI, navigation, room,
  wall, terrain-ground, sun-map, sky and fog state; `LevelChangeReady`
  hands the destination to the host.
- The viewer loads the destination through the same staged `LevelLoad` as a
  start-up load, keeps the surviving player (record, possessions, body) and
  moves it to the destination's start or the flow's `PlayerPlacement`; the
  level's music, ambience and reverb end and the model cache is cleared.
- The flow's `LoadLevel`, `TeardownWorld` and `PlacePlayer` steps are
  applied; the hub's match launch and return change level in process and the
  hand-off file (`arena_handoff`) is gone.
- Review: the game time stops from the change until the destination's
  collision is in (the surviving player fell out of the emptied world and
  died during the load); the old capsules' handles are removed before the
  level roots are despawned (a warning per placed character).
- Review: the practice next fight runs the practice scene filter after its
  reload, as the original's re-import passes it; an unused teardown copy and
  an unused query were removed; `--level-change level@frame,...` changes
  level unattended and logs entity, physics and asset counts per level.

## Evidence

The game-main unit's next-fight routine clears the scene and re-imports the
arena's stored level section through the practice filter callback (the save
unit's combatant preparation); its story respawn tears the world down,
clears the player pointer, loads `exanima05` through the save-or-archive
load and spawns the player again; its practice start loads `arena`/`arenae`
with the same filter. GPU check on the RTX 3090: `play exanima01`,
`play arena1 --opponent`, `play arenahub` and a scripted exanima01 ->
arenahub -> arena1 -> exanima01 change, compared with main.

## Out of scope

Exporting the current level into a save section and importing it on a
change, migrating followers, rebuilding the player from its record on a
respawn (the body survives as it is), the practice partner body, a
teleporter's landing from the device's placement, forced removal of shared
Bevy assets, and the map panel's level buttons (still a process restart).
