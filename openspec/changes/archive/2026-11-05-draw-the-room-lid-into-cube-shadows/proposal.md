# Proposal

## Why

docs/PLAN.md Phase 2 (rendering), the grid environment gap lane: the
round 45 leads "the lid in the lights' cube faces" and "callers for the
segment test". The room manager installs an omni-shadow hook that draws
the ceiling shell of the light's room into every cube face rendered for
it, and three callers outside the manager ask its shell ray query.
Exarust built the shells and the query but used neither.

Evidence (REA, Ghidra listing of `Exanima.exe`): the room manager's
constructor (the hook lists it fills), the shadow hook itself (light
kind test, world-point room query, front-face culling, the plain light
radius in the face program, the shell's own matrix), the cube face
renderer that calls the hook list after its casters, the light kinds the
light makers set (every point light, fires and the player's light
included, is the omni kind; only the sun is the direct kind), the
shell query and its callers: the reach test, the dragged item's
placement and the kinesis power's clear line and pick.

## What Changes

- Each light's cube faces take the black lid of the room holding the
  light (the world-point room query on the light's position), drawn
  after the casters with front faces culled at distance over R, no
  caster scale. A light that moves to another room, or that gets its
  lid when the shells arrive, renders its faces again.
  `--no-cube-lids` leaves the lids out, to compare.
- The level's rooms keep the shell strips, and `LevelRooms::shell_ray`
  asks the query on the rooms active now.
- The hand's reach test asks it with the cursor ray up to the picked
  point: an occluder of an active room in between puts the point out of
  reach (doors are exempt, as in the original).
- The kinesis's clear line from the head to its aim asks the occluders,
  then the lids, before the level.
- The frame profiler reports the lids drawn so far.

## Out of scope

- The dragged item's placement caller: Exarust's drop placement has a
  fallback (in front of the camera) the original does not have, so an
  occluded hit cannot be mapped to the original's "no placement" without
  reworking the drop. Left as a gap.
- The kinesis pick caller (Exarust hands the pick through the hand).
- The room-mode-off path (light aliases into one sector): every game
  scene sets room mode, so it is never reached in 0.9.5.

## Impact

`exa_render` (cube shadows), `exa_world` (room view, grid build),
`exa_game` (hand reach, kinesis), `exarust` (flag, profiler);
docs/RENDERING.md, crates READMEs.
