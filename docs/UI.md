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
  little fainter; see Compositing below);
- the reach test casts the pointer's ray at the level and measures the
  point from the player's aim point, 80 cm up (the original measures from
  the character's position at pelvis height; 3 m across and 3 m up or
  down); the original's test also has a 245 cm term not yet understood;
- Exarust's player walks to the cursor only outside combat mode for now, so
  chevrons show in combat mode only once it does there as the original;
- the state comes from the player's condition word
  ([formats/RSQ.md](formats/RSQ.md), Condition): dead (`0xA00`), collapsing
  (`0x800`), down for bit `0x200` (down, getting up, dormant), else
  standing; Exarust's fall shows the standing shapes until its balance is
  under 0.3, as the original's (its word is still normal then);
- nothing collapses yet: a killing blow kills at once, so the arrowhead
  takes the ring's place at the blow, where the original keeps the ring
  through the 3 s collapse; the crosshair has no use until interaction;
- `--cursor x,y` holds the pointer at a window position for unattended
  runs ([DEBUGGING.md](DEBUGGING.md)).

## Health bars

At the bottom centre of the game's view the original draws two framed bars
side by side: the player's health and, beside it, focus (the thaumaturgy
pool) in blue. With the window W × H physical pixels and the GUI scale g:

- **Place.** Each bar is 48·g wide; the health bar starts 50·g left of the
  centre (W/2, rounded down), the focus bar 52·g further right; both at
  y = H − 5·g. The fill covers rows y + 2 to y + 4·g − 5 (14 rows at scale
  5). In the user's 1600 × 1014 retail window: x 550–789 and 810–1049, rows
  991–1004, as on its screenshots.
- **Segments.** A bar takes two shares a and b of its width and fills
  three segments: to a, to b, and the rest. The pixel where the first ends
  is mixed from the first's colours towards the second's by how much of it
  the first leaves uncovered (the original draws it as a one-pixel line).
  Health: yellow to yellow / max, dark brown to red / max (damage that will
  heal), red beyond (lasting damage); no brown at red ≤ 0.01. Focus: blue
  to the focus, grey-blue to the focus capacity (spent focus that will
  return), purple beyond (capacity held by lasting powers); its blue is
  drawn only when at least a pixel wide.
- **Colours.** Each segment is two vertical gradients, the top half from a
  light colour to a dark one, the bottom half from a dark one back to a
  bright one, with the original's RGBA stops (top, middle, middle,
  bottom): yellow (240, 230, 200, 240), (120, 85, 15, 210), (130, 80, 0,
  200), (210, 180, 30, 240); brown (180, 160, 145, 200), (70, 50, 10, 160),
  (70, 40, 0, 128), (100, 80, 50, 200); red (220, 160, 130, 220), (100, 25,
  5, 210), (90, 0, 0, 190), (140, 60, 15, 220); blue (150, 180, 230, 240),
  (20, 60, 160, 210), (0, 30, 80, 200), (40, 120, 255, 240); grey-blue
  (135, 140, 150, 200), (30, 40, 50, 160), (5, 10, 40, 128), (70, 90, 120,
  200); purple (180, 170, 220, 220), (60, 20, 100, 210), (30, 0, 70, 190),
  (100, 40, 200, 220).
- **Inner shadow.** Over the fill, black at 240 along the bar's edge (the
  rectangle from y + 1, 4·g − 4 tall) fading to nothing g pixels in.
- **Frame.** Two one-pixel rings round the rectangle from (x − 2, y) to
  (x + w + 2, y + 4·g − 2): the outer in (129, 134, 135), the inner in
  (28, 30, 30), both at 239, each multiplied by the stone texture
  `guitex.rfi` (512² in the resource pack, mean (111, 113, 117)) sampled
  one texel per window pixel, rows counted from the window's bottom. The
  colours are the HUD skin's palette (its scheme's base (97, 104, 105)
  scaled by 1.3 and desaturated by 0.4, and by 0.29).
- **Drop shadow.** Black outside that rectangle, 2·g pixels wide, at
  220 × (1 − d / 2g)⁴ with d the distance from the rectangle (round at the
  corners).
- **Order.** Both frames, both drop shadows, the health bar, the focus bar.

**Fade.** The bars show while combat mode is on, and in any frame in which
the drawn yellow health differs from the last frame's or the focus has
dropped (the original also shows them while a power is cast, for 1 s after
red health is restored or a quick-bar power is used, and for 5 s when the
player interacts with its own body; Exarust has none of these). They fade
in over 500 ms; once no longer shown they stay 200 ms and fade out over
2 s (game time; from a partial fade they rise or fall from where they
are). The fills blend at the fade squared, the frames and shadows at the
fade. A single frame of change shows nothing: what brings the bars up out
of combat mode is a change frame after frame, such as the original's
yellow health regenerating after a blow. Exarust's yellow health does not
regenerate yet, so out of combat mode a blow alone shows no bars.

**Stamina gauge.** With a shield in combat mode a thinner bar shows above
the others, 48·g wide and centred, 2·g tall at y = H − 9·g, with its own
fade by the same rule: block stamina, white to grey for what is left and
grey to black for what is used, with a three-pixel inner shadow (g/2 + 1)
and the same frame drawn over it.

Exarust (`exa_ui::hud` draws, `exa_game::hud` reads the player):
- the bars are painted on the CPU in the original's order and blended in
  gamma space, then shown as one image node in physical pixels, repainted
  only when the values, the fade or the window change;
- Exarust has no powers: focus and its capacity are 1, the full blue bar
  the original shows for a character who casts nothing. It has no shields:
  the stamina gauge never shows;
- the original offsets the stone by the last form it drew; on a lossless
  capture of the user's running game the offset is zero (the window's
  bottom-left corner, as Exarust samples it): its frame lines follow the
  texture texel by texel and differ from the rule above only by the
  floor's share at opacity 239;
- on the retail values (`--health 0.176,0.226`) the bars match the
  reference screenshot within about 5 of 255 down every column of the fill
  ([DEBUGGING.md](DEBUGGING.md)).

## Heartbeat

When a blow that does not kill leaves the player's yellow health below
0.05 (at 0.01 or more), the original starts the heartbeat, at most once a
minute, in story mode only (never in practice or the arena): for 7 s a
dark red (80, 0, 0) border lies round the view, strongest at the window's
edges and fading to nothing a fifth of the window's height in (on the
sides too, measured by the height), pulsing every 800 ms from 0 to an
opacity of 96 at the pulse's middle (opacity (1 − |400 − t mod 800| /
400)² × 96, t in ms of real time). Meanwhile the `heartbeat` sound loops at
gain 0.3. Both stop after 7 s (menus and level changes fade them over 1 s).
An item's "second wind", when ready, fires instead.

Exarust (`exa_ui::heartbeat` draws, `exa_game::hud` starts it and plays the
sound): every level but the arenas counts as story mode; `--heartbeat`
allows it anywhere. The border is drawn from an image an eighth of the
window's size, linearly stretched.

## Compositing

The original's interface blends sRGB values; Bevy blends in linear light.
Interface images painted on the CPU (the health bars, the heartbeat's
border) are composited as the original blends, and each pixel's opacity is
then fitted so that Bevy's blend gives the original's result over the
arena's dark floor (40 of 255): exact there, black within 3.5 levels over
floors from 10 to 60, brighter ground showing through a little more.

The interface camera draws into its own texture, cleared to transparent
black, which then goes over the game's view. Its colours there are already
multiplied by their opacity, so it is laid over premultiplied. (Bevy's
default for a later camera, straight alpha, multiplied every translucent
pixel twice: half-transparent white came out at 87 of 255 over the floor
instead of about 150.)

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
