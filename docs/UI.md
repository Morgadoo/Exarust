# User interface

What the interface draws and how it matches the original. Code: `crates/exa_ui`.

## Fonts

The base UI font is the texture `fontbase24r` in the resource pack:
- a 1024 × 1024 A8 atlas: a 16 × 16 grid of 64-pixel cells, one per Latin-1
  code (row = code / 16, column = code % 16, top row first);
- each glyph is stored upside down in its cell, and all cells share one
  baseline;
- glyphs are proportional. Exarust takes a glyph's advance from its inked
  columns plus a 4-pixel gap, and gives a space 18 pixels. These are
  measured guesses until the original's glyph metrics are found.

The font is read from the installation at run time and never shipped.

## Frame rate

The original shows the frame rate in the window's top-right corner, as in
"57.25 FPS": light grey, in the base font at its 24-point size (0.47 window
pixels per atlas pixel). The original draws in physical pixels whatever the
display's scale factor, and Exarust does the same.

Exarust:
- shows the mean rate over each half second, with two decimals;
- hides the readout with `--no-fps`.

## Loading screen

Exarust's own, not the original's: while a level or play mode loads, the
window shows a dark page with "Loading <level> - <stage>" in the base font
and a progress bar. The window answers the desktop only between frames, so
no frame of the load may take long: GNOME offers to force-quit a window that
has not answered for about five seconds. The work is spread out, and what
cannot be cut into short pieces runs on the async compute pool:
- the level is built a part at a time (`exa_world`'s `LevelLoad`), 25 ms of
  building per frame: the grid's wall and floor sets (their materials and
  textures warmed over as many frames as they take) and rows of cells, the
  terrain a tile or a palette material's textures at a time (its packed
  surface map on the pool), the level's own model, props, placed objects,
  then the lights. A model's mesh groups are built on every core;
- play mode builds the level's collision next (`exa_physics`'s
  `StaticBuild`): 4 ms a frame copies meshes, the triangle-mesh colliders
  are built on the pool, before the player and the level's characters are
  spawned on it;
- their apparel models are fitted to the body and their apparel painted on
  the pool; their materials and the painter's textures load one at a time
  within 8 ms of dressing a frame, the painted textures become GPU textures
  32 MiB a frame at most (a crowd's at once stalled the upload for hundreds
  of milliseconds), and a body is dressed once all of it is in. Dressing
  and spawning run before the animation systems, so a skinned mesh is bound
  to its skeleton in the frame it appears: one drawn unbound fails wgpu
  validation (a skinned pipeline with `model_only_mesh_bind_group`);
- last, the screen waits for the lighting: the GI source collected and the
  voxel GI block around the camera built, both on the pool, and its 600-step
  warm-up, run 16 steps a frame. The first frame shown is lit as the
  original's is after its load. With `--no-gi` there is nothing to wait for.

Every frame of the load is timed: one longer than 250 ms is logged as a
warning with its stage and the main world's share of it (the rest waits
for the render world, the GPU and the window system), each stage's time is
logged at debug level (`RUST_LOG=exa_ui=debug`), and the whole load's time
when it ends. The 10 s after the load (or after the start, when nothing
loads) are watched the same way, and their longest frame logged.
Scripted runs start at the end of the load: `--frames` (before a
screenshot) and `--shove` count frames from there, and `--autowalk` starts
walking then.

The screen is hidden when the load ends, not despawned. Its camera, the
highest-ordered, stays on without clearing: it draws the rest of the
interface (the frame rate) over the game's view, and Bevy draws no
interface while its interface camera is off.

The window's app id (X11 class) is `exarust`, so the desktop names it in
its dialogs.
