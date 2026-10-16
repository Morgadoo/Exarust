## Context

What makes the original's gaps black. The frame is cleared to the
environment's background, (0, 0, 0, 1) in every tile-grid level (indoor
preset; read live in the arena hub). Before anything else, a pass hook of
the room manager draws, with colour writes off and a flat program, for
every room whose state is not hidden: its occluder shell always, and its
ceiling shell when the room is not fading. Everything under those shells
then fails the depth test and keeps the clear colour. The shells are
position-only meshes built at load from the set pieces: each placed
piece's `[…` children (boxes from the floor to 2.7 m that fill the 55 cm
between a 20 cm facade and the cell edge) go to the room's occluder, and
its `+R…` children plus the sets' ceiling slot pieces (flat at 4 m,
ramping to 2.64 m at the cell edge) to the room's ceiling. A last pass
draws, blended (SRC_ALPHA, ONE_MINUS_SRC_ALPHA) with the depth test off,
the ceiling shell of every room fading in (alpha 1 − (now − t)/1000 ms,
done below 0.01) or out (alpha (now − t)/1000 ms).

Which rooms. Each frame a tracker takes the cell of the camera's target
(the player; the cell is the floor of x/6 m, plus half the grid). Wanted
are its room and, for each door link starting in that cell, the room
behind it when the point a quarter of the way from the cell's centre to
the neighbour's centre is within 2 m of the target. The active list keeps
its order; a room no longer wanted starts leaving (an entering room
mirrors its start time to 2·now − (t + T) so its alpha carries over), a
leaving room is dropped after 1 s (its door neighbours' states set to
hidden), a leaving room wanted again turns back. Every list room sets its
door neighbours adjacent, then itself active. States are sticky: only the
drop step lowers them. Hidden rooms are not drawn at all; adjacent rooms
are drawn through their portals with their lid in depth.

Rooms come from a flood fill of plane A at load: the seed is the first
open cell of the last row that has one; door seeds wait on a stack, last
in first out; a room spreads over every edge but walls (7) and doors (B);
a closed cell reached that way joins without spreading; a door records a
link when the cell behind it is open and does not face it with a wall.

Terrain in a grid level. The grid loader sets the scene's grid mode for
every level with a grid chunk, the outdoor arenas included. In grid mode
the terrain builds a 6 m block only where plane A has `0x800000`, block
(bx, by) of tile (tx, ty) being cell (x0 + 8·tx + bx, y0 + 8·ty + by) with
x0 = (grid width − 8 × tiles across)/2 halved towards zero. Each block node
goes to the room of its bounds' centre, else to the default sector.

The research notes, with addresses, are git-ignored:
`context/re/black-fill.md` (§1–§8 and the Plan, §P0–§P8). The pages this
change extends are [`docs/formats/RFC.md`](../../../docs/formats/RFC.md)
(Grid building) and
[`docs/formats/TERRAIN.md`](../../../docs/formats/TERRAIN.md).

## Goals / Non-Goals

**Goals:**
- black wall hollows, black lids over the rooms behind the player's
  doors, nothing of the other rooms, and no terrain outside the flagged
  cells, as the original shows them from the gameplay camera;
- the original's room map, door links, active rooms and 1 s fades;
- lights, shadows, GI and collision unchanged.

**Non-Goals:**
- portal frusta and scissor for adjacent rooms;
- the ceiling shell in omni shadow faces; light reach through rooms;
- doors' open state; ground queries in absent terrain blocks.

## Decisions

- **Opaque unlit black shells instead of a depth-only pre-pass.** Bevy's
  opaque phase does not guarantee that the shells draw first, which a
  depth-only pass needs. Opaque pure black gives the same image over a
  black clear, and every grid level clears to black. The overlay is a
  blended material with depth compare Always and no depth write, sorted
  after every other blended surface by a large depth bias.
- **A material of its own** (`ShellMaterial`), so the shells stay out of
  the surface material's systems (fade to black, light slots, GI) and
  never cast shadows.
- **Only meshes are hidden.** A hidden room's mesh entities get
  `Visibility::Hidden`; lights, GI emitters, shadow links and colliders
  stay. Characters are hidden whole by the room of their position.
- **Rooms of static content** come from the world bounds of each top-level
  root's meshes (a prop under the level root, a placed object, each top
  node of the level's own model), one frame after it spawns; props under
  props inherit, and meshes added later (flames) take their root's room.
- **The shells are not under the level root**, so the static collider
  build leaves them out.
- **The fly viewer** follows the point 20 m ahead of its camera, the
  original camera's distance to its target, so retail poses give retail
  rooms. Without `--camera` (the whole-level overview) it shows every
  room; the original has no such view.
- **The fade clock** is real time [guess: the original's clock was not
  traced].
- **Set pieces are top-level nodes only.** A piece's `+R…` child
  (`+RBA02.531` under a door piece) had overwritten the set's ceiling
  slot piece, so the lids had triangle holes; the data has every `+W`,
  `+C`, `+T` and slot `+R?` piece at the top level.

## Open questions and how they were answered

- Anchor, eye or target? REA: the camera's target node; the live d1 and
  c3 views and frame b9 agree.
- Which levels are in grid mode? REA: the grid chunk's loader sets it
  unconditionally; corpus: the outdoor arenas' grids flag exactly their
  playable rectangle (arenao3 16 × 15 cells).
- Does the cap path add the end walls' extra strips? Not read; no cap
  cell with edge code 1 or 2 exists in the corpus. The cap uses the same
  slot choice as a normal cell.
- The fade clock (game time or real time) and whether plants follow the
  terrain blocks: guesses, labelled as such in the docs.

## Risks / Trade-offs

- [Without portal clipping an adjacent room can show past its lid at
  oblique views, and a band between the tavern set's short occluders and
  the cell edge can show from a hidden room's side] → out of scope,
  noted in RENDERING.md "Not done yet".
- [A level clearing to a fog colour would show black shells where the
  original shows fog] → no grid level does; the outdoor arenas have no
  wall sets and so no shells.
