## Why

PLAN Phase 2 ("Rooms and portals", M2 walkable world), component
TGridEnvManager. Its routines were listed against Exanima.exe one by one:
the room walk, the fade pass and the depth pre-pass are ported, but two
numeric details of the walk and the fade differed, and the room manager's
cell active query (a cell's room has faded in) had no live caller. The
original's only caller is the story's exploration: once a second the 5 × 5
cells around the player whose room is active earn discovery points. The
running story only converted points; no cell was ever explored.

## Evidence

REA (Exanima.exe), read against the assembly:
- The per-frame room walk takes the anchor's cell as `row × width + column`
  accepted from 0 to below the cell count, and compares a door point's
  squared distance to the anchor (the vector helper's x² + z²) with the
  squared 200 cm reach.
- The fade pass computes the elapsed time as a signed 64-bit difference,
  divides by the 1000 ms length in double, takes `1 − t` in double, rounds
  it once to single, widens that single again and compares it with the
  double 0.01 (`COMISD`, a NaN draws); below it the room turns active and
  is not drawn. Leaving rooms draw `t` rounded to single. The alpha is not
  clamped; the 8-bit colour target clamps it.
- The cell active query reads the cell's room word and the room's fade
  state low bit (faded in, or wanted again mid fade-out).
- The experience unit's tick walks the 5 × 5 cells (clamped inside the
  grid's border) around the player's cell when the scene is in grid mode
  and the room manager exists, and asks that query for each open (plane A
  0x80000) cell not yet in the level's bitmap. The level-enter hook finds
  the level's bitmap by its 16-byte name or adds one at the grid's size.
- The level names the story passes are the lowercase locale names
  (`exanima04`, `exanima06` in the save unit's data).

## What Changes

- exa_world: `RoomTracker::wanted_through` takes the linear anchor cell and
  the squared reach; `RoomTracker::overlay` follows the fade pass's double
  arithmetic and threshold; `Rooms` keeps the grid's size and each cell's
  plane A word.
- exa_game: `StoryExperience::explore` and `RoomCells`; the live discovery
  pass explores around the player before it converts, entering the level's
  map by its locale name.

## Out of scope

- The room walk's forced-room branch (the arena's preview pins a room when
  the rendered view is not the main one) and its "still fading" output
  byte: Exarust has neither caller.
- The depth-only form of the shell pre-pass and the views other than the
  main one (Exarust draws the shells opaque black over the black clear, the
  same picture; `docs/RENDERING.md`).
- The room mode without rooms (one light alias per sector) stays
  unreachable: every 0.9.5 grid level builds rooms.
- Item-kind and kill discovery.
