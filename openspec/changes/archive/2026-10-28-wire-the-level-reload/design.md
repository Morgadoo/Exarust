# Design

The original destroys its scene and builds a new one from the save or the
archive, then spawns the player from its record. Exarust keeps the player's
entity subtree (its record, possessions and body are components) and
rebuilds only what the level owns, which keeps the session state without a
record round trip.

- Ownership is a marker: `LevelEntity` on each root a level spawns. The
  teardown despawns those roots recursively; everything else (player,
  camera, interface) survives.
- The physics world is replaced rather than emptied collider by collider:
  all fixed geometry and props belong to the level, and the only surviving
  bodies are controlled characters, whose `CharacterHandles` are removed so
  the fixed step makes new capsules. The handle maps (`ColliderNodes`,
  `StandIns`, `PendingKinematics`, `StaticBuild`) are reset with it so no
  stale handle can alias a body of the new world.
- The teardown runs in `LevelChangeSystems::Teardown`; the viewer's
  `apply_level_change` runs after it and starts the staged load, so several
  steps in one frame never start two loads. A teardown in the same step
  batch as a load is folded into it.
- The viewer pauses `Time<Virtual>` when it starts the destination's load
  and gives it back (unless it was already paused) once the collision is in
  and the player is moved; a level missing from the pack gives it back at
  once.
- `PlayerPlacement` waits for the loading screen to close, so the player is
  placed on the new level's collision.
