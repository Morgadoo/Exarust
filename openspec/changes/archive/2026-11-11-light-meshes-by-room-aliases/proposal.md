# Proposal

## Why

Phase 2 (static world) of docs/PLAN.md, lights. The room manager's light
lists (`exa_world::rooms::RoomLights`) were modelled but the renderer did
not read them, and the live scene kept its lights global: any light linked
to any mesh it reached and whose flood window it passed, whatever sector the
mesh was registered in. The original lights a mesh only through its
sector's light aliases.

Evidence (REA, `Exanima.exe`):

- the room manager's constructor installs, on the scene and on every room
  sector, a node hook (a node moves into the room of its cell) and a cell
  hook; the room sectors carry the grid bit;
- the light activation pass makes the light's alias in its own sector and,
  for a grid sector, stores the light's cell and floods an omni light (the
  directional fill otherwise); the non-grid spread along linked sectors is
  not taken;
- the flood's cell-record step calls the scene's cell hook with the light
  and the cell each time it records the light in a cell (a full cell that
  does not list it calls nothing);
- the cell hook finds the cell's room and makes the light's alias in that
  room's sector (or stamps the existing one current) and marks the sector's
  "light added" bit;
- a mesh's light bucket list is rebuilt from its sector's alias list, one
  bucket per alias, so its lights are its sector's aliases;
- the only alias makers in the binary are the activation, the cell hook,
  the find-or-create used by the non-grid spread, the directional fill and
  the batch insertion of a scene without the grid bit, and the room-mode-off
  hook; nothing passes room lists through the doors at load.

## What Changes

- The live room light lists: each light placed in its sector (the room of
  the cell it stands in, else the scene's own sector) and added to the room
  of every cell its flood recorded it in, rebuilt whenever it floods.
- The light links: a mesh registered in a room sector (or the scene's own
  sector) links only lights listed there; terrain tiles skip the test as
  they skip the wall test. A mesh moved to another sector is relinked.
- `--no-room-lights` turns the test off to compare; a periodic log line
  reports the lists and the links the test refused.

## Out of scope

The load-time `RoomLights::propagate` stays for the terrain's hidden-block
ranking; meshes that cast no shadows still take every light in reach; the
sector a grid piece is registered in is still its building cell's room.
