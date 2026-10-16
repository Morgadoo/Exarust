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

## Interface scale

The original sizes its interface by a whole-number GUI scale, picked again
whenever its window changes size, from the window's height in physical
pixels:

| height | under 700 | 700 | 870 | 1051 | 1390 | 2080 |
|---|---|---|---|---|---|---|
| scale | 3 | 4 | 5 | 6 | 8 | 12 |

A hidden setting (`GUISize`) moves the pick up or down the row; it is 0
unless set. The user's retail window, 1014 pixels tall, has scale 5, read
from the running game. Exarust (`exa_ui::scale`, `GuiScale`) does the same
with its window's physical height; its default 1600 × 900 window gives 5.

## World cursor

In the game's view the original hides the system pointer and draws its own
cursor at the pointer's position: a flat sprite, not a decal. Over the
steep chase camera it looks as if it lay on the floor (the red ring and red
arrow of the reference screenshots).

- **Sprite.** `cursors.rfi` in the resource pack: a 256 × 512 coverage
  (A8) atlas of 2 × 4 frames, 128 pixels each, stored bottom row first
  like every texture (a frame's first stored row is the drawn sprite's
  bottom). Stored rows, top first: the arrowhead and a crosshair of four
  chevrons; two chevrons and a four-armed curved cross; one chevron; the
  ring. Two frames are empty.
- **Size.** s = round(3 × GUI scale × cursor size) × 2 physical pixels,
  at most 128; the cursor size setting is 1, so 24 pixels at scale 4 and
  30 at 5. The atlas is resampled to that size when it changes (Exarust
  averages each new pixel's area).
- **Colour.** Red (255, 76, 76) at alpha 180 in combat mode, amber
  (255, 205, 76) opaque outside it. Once any shape but the ring or the
  pointer has been drawn in combat mode, the red stays opaque for the rest
  of the session (the original writes 255 into its red's alpha).
- **Outline.** Four black copies one pixel left, up, down and right under
  the sprite, at the colour's alpha squared (127 for the red, 255 for the
  amber).
- **Turn.** The ring and the chevrons turn so that their top points away
  from the player's point on screen: walking chevrons point where the
  player goes. The arrowhead is turned 33° counter-clockwise and moved
  0.275 s right and 0.4 s down, which puts its tip on the pointer's
  position, as a system pointer's. The other shapes stay upright.

The shape follows the player's state (the original's player behaviour and
its movement request):

| | combat mode | outside it |
|---|---|---|
| right button held (walking to the cursor), however near | one chevron, two beyond 300 cm | the same |
| Ctrl (interact) held | the arrowhead | — |
| standing | the ring | the arrowhead over a point within 3 m of the player, else the cross |
| down after a fall | the ring | the cross |
| collapsing from a lethal wound | the ring | the cross |
| dead | the arrowhead | the cross |

Seen in the user's retail game: the ring in combat, also while down and
collapsing; the chevron on the right button; the arrowhead after each of
three deaths in combat mode; the arrowhead and the cross between fights.

In combat mode the original aims at the pointer's point on the plane 80 cm
above the player's ground (walking to the cursor and the attacks' heading,
`docs/PLAYER.md`). An option that is off by default would also pin the
cursor to that plane and move the system pointer with the camera; Exarust
leaves it out, and never moves the pointer.

Exarust (`exa_ui::cursor` draws, `exa_game::cursor` picks the shape):
- the system pointer is hidden only while the world cursor is drawn: in
  play mode with the chase camera, after the load, while the pointer is in
  the window;
- Bevy blends in linear light, the original's interface in gamma space.
  Exarust remaps the opacities so that the result is the original's: the
  outline's exactly over any background, the sprite's exactly over black
  and closely over the dark floors (over bright ground it comes out a
  little fainter);
- the reach test casts the pointer's ray at the level and measures the
  point from the player's aim point, 80 cm up (the original measures from
  the character's position at pelvis height; 3 m across and 3 m up or
  down); the original's test also has a 245 cm term not yet understood;
- Exarust's player walks to the cursor only outside combat mode for now, so
  chevrons show in combat mode only once it does there as the original;
- nothing collapses yet: a killing blow kills at once, so the arrowhead
  takes the ring's place at the blow, where the original keeps the ring
  through the 3 s collapse; the crosshair has no use until interaction;
- `--cursor x,y` holds the pointer at a window position for unattended
  runs ([DEBUGGING.md](DEBUGGING.md)).

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
interface (the frame rate, the world cursor) over the game's view, and
Bevy draws no interface while its interface camera is off.

The window's app id (X11 class) is `exarust`, so the desktop names it in
its dialogs.
