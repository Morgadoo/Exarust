## Why

PLAN Phase 3 (player control and camera). The navcam lanes rebuilt the
camera controller's follow branch (TNavCameraControl). Its known gaps: the
tick's free-flight branch, which moves the looked-at point of a camera
that follows nothing (reached by the spectating camera), and the grid
tileset loader's mark, without which the dungeons' set pieces were never
wall-probe obstacles.

## Evidence

REA (Exanima.exe), read in the assembly where the numbers matter:
- The controller's tick, free branch: the movement keys (preset `0x800`,
  the viewport field −1, a never-written global byte clear) add and
  subtract the drawn right and back rows, the pad routine's movement
  vector is added, `0x20000` flattens and normalises past 2⁻²³; the pull
  is `(stored shown zoom / largest distance + 0.25)·extent`; the terrain's
  height query (the scene's terrain object) lifts the point; the drift is
  the point's move since the last tick, a pan past 2⁻²³ adds the pull along
  it, `0x2000` brakes; a drift past 1e-5 (squared, double) moves the point
  by the rate field (0.01^(1/59)) and clamps its height to the preset's
  0–200 cm. The follow-point spring (0.0653) and the area bounds run only
  under `0x400000`, the editor preset's flag.
- The frame placement, unfollowed: the point drawn between its last two
  ticks (`a + (b − a)·t`) plus a height offset (a vector easing toward a
  camera-node field) and the preset's offset field.
- The wall routine returns without a follow object; its start is the
  followed object's point, for a character its render-interpolated hip,
  50 cm lower, in its parent's frame.
- The scene walk skips a node whose mark word has bit 0: the scene unit's
  visible pass fades a node with that bit out, so it is the cut-away mark.
- The tileset loader (wall and floor sets alike) ORs `0x10000000` into
  every node's second flag word unless its name starts with the byte
  0xE4, and `0x1000` into its class word.
- Corpus (`grid::tests::shipped_tileset_probe_marks`): 8 393 set meshes
  become obstacles (one of kind `0x5…`), 280 are 0xE4 pieces, 37 lack the
  scene walks' bit; no set mesh is a cut-away candidate.

## What Changes

- exa_game: `nav_camera::FreeFocus` (the free tick and frame point),
  `free_pull`; the chase camera runs the free branch while spectating with
  no body followed (W/A/S/D, the terrain lift), draws its own point, runs
  no wall probe then, starts the probe at the followed hip, and skips
  obstacles the cut-away fade marked; the arena's spectated start places
  the camera through the placement routine and frees it there.
- exa_assets: `mark_tileset`, `ModelLoader::tileset_model`.
- exa_world: the grid loads its sets through `tileset_model` and attaches
  `WallObstacle` to set pieces.
- docs/PLAYER.md (Camera), crate READMEs.

## Out of scope

The key gate's viewport field (not identified: Exarust reads the keys
whenever the camera is free); the pad's movement vector; the frame's
height offset; the editor preset's follow-point spring and area bounds;
the class word's `0x1000`.
