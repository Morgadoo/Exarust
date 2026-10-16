## Why

The user's screenshots set Exarust beside the original: in the original
the space between and inside walls, the rooms the player cannot see and
the rock between rooms are solid black. Exarust shows dirt and floor
there. The original paints nothing black. It clears the frame to black
and leaves three things undrawn, and Exarust draws all three:

- the hollow inside every wall, which the original fills in depth with
  the pieces' `[…` occluder boxes, merged per room;
- the rooms the player is not in: rooms behind a door of the player's
  room are covered by their black lid (the merged `+R…` ceiling strips)
  and every other room is not drawn;
- terrain outside the cells the grid flags as terrain-floored, which the
  original never builds in a grid level.

Phase 2 (Static world), milestone M2 "walkable world" (PLAN: rooms and
portals).

## What Changes

- Rooms: the open cells of a tile grid are flood-filled into rooms joined
  by door links, by the original's rules (seed, door stack, link test).
- Shells: every piece's `[…` children are merged per room into a black
  occluder, and the sets' ceiling slot pieces and the pieces' `+R…`
  children into a black lid; neither is spawned as scene geometry. Open
  cells without a wall set that carry the terrain flag add lid strips
  from the first wall set (ceiling caps). Only top-level set nodes are
  pieces: a piece's `+R…` child no longer takes the place of the set's
  ceiling slot piece.
- Room view: the player's room and the rooms behind a door of its cell
  whose point 1.5 m inside the cell is within 2 m of the player are
  active, fading in and out over a second under a black overlay of their
  lid; their door neighbours are adjacent, drawn under their black lid;
  every other room is hidden. The fly viewer follows the point 20 m
  ahead of its camera when `--camera` places it; its overview and
  `--all-rooms` show every room.
- Terrain in a grid level: a 6 m block exists only where its grid cell
  carries the terrain flag; each block's mesh and plants go to its room.
- Shells are drawn opaque, unlit pure black (no shadows, GI or light
  lists), which equals the original's depth-only shells over its black
  clear; lights, shadow links, GI and collision are unchanged.
- `--ceilings` goes (the lid replaces it).

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, the
  disassembly): the shell pre-pass and the fade overlay, the room
  tracker and its state machine, the flood fill and door-link test, the
  ceiling cap, the terrain presence bits, and the grid loader setting
  grid mode for every level with a grid (`context/re/black-fill.md`
  §1–§8, Plan §P1).
- The running original, read only: its room map and active rooms in the
  arena hub match the rules exactly (10 rooms, 18 links, active
  {2, 1, 7} and {2, 1} at two live views) and screenshots at three
  camera poses.
- Corpus: the wall sets' `[…` and `+R…` nodes (`exa-tool export-gltf`),
  the grids' planes of every level (`exanima01`: 153 rooms, 356 links,
  682 terrain cells).

Out of scope:
- portal frusta and scissor for adjacent rooms (an oblique view can show
  a sliver of an adjacent room past its lid's edge);
- the lid drawn into omni shadow faces; light reach through rooms;
- door bodies and their open state (every door counts as open);
- height queries and collision of absent terrain blocks.

## Capabilities

### New Capabilities
- `room-visibility`: rooms, the active/adjacent/hidden rooms around the
  player, their fades, the viewer's anchor and the debug view.

### Modified Capabilities
- `grid-building`: occluders and ceilings become per-room shells; ceiling
  caps; ceilings on demand go.
- `terrain`: terrain blocks in grid levels.

## Impact

- `exa_world`: new `rooms` (flood fill, tracker) and `room_view` (the
  plugin); `grid` (shells, caps, top-level pieces, rooms on parts),
  `terrain` (blocks, parts per room), `lib` (rooms at the grid stage,
  rooms of props, placed objects and the level model; no ceiling option)
- `exa_render`: new `shell` material (opaque black, fading overlay)
- `exarust`: `--all-rooms`, no `--ceilings`, the anchor on the player
  and ahead of the fly camera, characters tracked
- Docs: `docs/RENDERING.md` (Rooms and black fill),
  `docs/formats/RFC.md` (Grid building), `docs/formats/TERRAIN.md`,
  `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, the `exa_world` and
  `exa_render` READMEs
