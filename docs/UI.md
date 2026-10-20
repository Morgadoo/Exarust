# User interface

What the interface draws and how it matches the original. Code: `crates/exa_ui`.

## Fonts

The base UI font is the texture `fontbase24r` in the resource pack:
- a 1024 × 1024 A8 atlas: a 16 × 16 grid of 64-pixel cells, one per Latin-1
  code (row = code / 16, column = code % 16, top row first);
- each glyph is stored upside down in its cell, and all cells share one
  baseline;
- glyphs are proportional. The one-line readouts (frame rate, loading
  caption) take a glyph's advance from its inked columns plus a 4-pixel
  gap and give a space 18 pixels, a measured guess.

The font is read from the installation at run time and never shipped.

The original's metrics, read from `Exanima.exe` (`exa_ui::gui_font`): the
interface has six fonts, `Rayform06R`, `Rayform08R`, `Rayform10R`,
`Rayform12R`, `Rayform16R` and `fontbase24r`. Each records the quad a
glyph is drawn as (8 × 13 up to 36 × 48 pixels, the cell's top-left
corner of the texture), the height rows centre text by, the slant of its
italic (1 to 8), a gap (6, 8, 10, 12, 17, 24) and a line height (12, 16,
20, 24, 36, 48), and one advance per byte, stored as an offset that the
original adds the gap to once at start-up. In `fontbase24r` a space
advances 21, "A" 28, "i" 12 and "m" 37. A glyph's texture cell is its
column and row over 16; the quad's top samples the cell's far edge (the
glyphs are stored upside down).

A GUI scale level (0 to 5, the index of the scale in the table below)
sets the GUI unit, the main and small fonts and the cells of the two icon
atlases:

| level | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| unit | 3 | 4 | 5 | 6 | 8 | 12 |
| main font | 06R | 08R | 10R | 12R | 16R | fontbase24r |
| small font | 06R | 06R | 08R | 10R | 12R | 16R |
| large icons | 16 | 20 | 28 | 32 | 42 | 64 |
| small icons | 12 | 16 | 20 | 24 | 32 | 48 |

Before a level is chosen the unit is 4 with `Rayform08R` and
`Rayform06R`. Exarust keeps the level with the scale (`GuiLevelState`).

## Interface scale

The original sizes its interface by a whole-number GUI scale, picked again
whenever its window changes size, from the window's height in physical
pixels:

| height | under 700 | 700 | 870 | 1051 | 1390 | 2080 |
|---|---|---|---|---|---|---|
| scale | 3 | 4 | 5 | 6 | 8 | 12 |

A hidden setting (`GUISize`) moves the pick up or down the row; it is 0
unless set. Its handler keeps 0 and negative values, but turns a positive
one into 1 only on a display 800 pixels tall and into 0 on any other
([formats/SETTINGS.md](formats/SETTINGS.md)); Exarust's launcher applies
the settings file's value with the primary display's height. The user's retail window, 1014 pixels tall, has scale 5, read
from the running game. Exarust (`exa_ui::scale`, `GuiScale`) does the same
with its window's physical height; its default 1600 × 900 window gives 5.

## World cursor

In the game's view the original hides the system pointer and draws its own
cursor at the pointer's position: a flat sprite, not a decal. Over the
steep chase camera it looks as if it lay on the floor (the red ring and red
arrow of the reference screenshots). While Cursor Look is held the game
captures the mouse: the system and world cursors are both hidden until it
is released. While Exarust's map panel is open (F9,
[DEBUGGING.md](DEBUGGING.md)) the system pointer is shown instead of the
world cursor.

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
- a killing blow starts the 3 s collapse ([formats/RSQ.md](formats/RSQ.md),
  The collapse), so in combat mode the ring stays through it and the
  arrowhead comes at death, as in the original; the crosshair has no use
  until interaction;
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
of combat mode is a change frame after frame, such as yellow health
regenerating after a blow ([formats/OTHER.md](formats/OTHER.md),
Regeneration): out of combat mode, a wounded player's bars show while
the yellow refills over the brown, up to 50 s and more, and fade once it
reaches the red. (The original draws yellow interpolated between body
ticks; Exarust draws the tick's, so above 83 frames a second some frames
repeat it, which the 200 ms hold covers.)

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
An item's second wind, when the game clock is past its ready clock, fires instead: it waits 180000 ms, and the heal sound uses volume bits 0x3dcccccd and pitch bits 0x3f800000. At that ready clock the heartbeat starts. While the wind is active, each living health tick adds the double 0.0003 to yellow. The wind stays through the ready clock minus 180000 ms plus 3000 ms, and the next tick still adds and then ends it.

The trigger is the damage call's low-health check
([formats/OTHER.md](formats/OTHER.md), Applying damage), not a yellow
threshold: it also runs after non-vital damage that leaves yellow at 0
and after a blow that deals nothing to a player already under 0.05.

Exarust (`exa_ui::heartbeat` draws, `exa_game::hud` starts it on the
`exa_combat::Weakened` message for the player's body and plays the
sound): every level but the arenas counts as story mode; `--heartbeat`
allows it anywhere. The border is drawn from an image an eighth of the
window's size, linearly stretched.

## Death fade

When the player falls to its death ([formats/RSQ.md](formats/RSQ.md),
Falling to death), the original's session starts a fade to black at the
pit's sound, 1 s after the player is lost: a black rectangle over the
whole window, drawn before the health bars (so they stay over it), its
opacity round(t × 255) with t the share of 3 s of game time gone (the
clock that stops while the game is paused), at most 1. The world's sounds
fade with it (1 − t), and when it is complete the game pauses (about 4 s
after the loss, when the player dies and leaves the world).

Exarust (`exa_ui::fade` draws `DeathFade`, `exa_game::fall` starts it):
the black layer lies over the heartbeat's border and under the bars (Z
895), its opacity on Bevy's virtual time, remapped for the original's
gamma-space blend; hidden while a level loads. Not yet: the world sounds'
fade and the pause at the end (Exarust has no death menu to show then).

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

## Speech bubbles and the reply panel

The logic is in `exa_script` (`bubbles`, `dialogue`); nothing draws it yet.
A spoken line shows for 1000 ms + 50 ms per byte, the narrator's under 10 s
stretched by (2 − d/10 000), the player's halved. A speaker's new line closes
its old bubble (fading four times as fast) and waits 375 ms past its end; a
character's line is cut at a CR and the rest follows as another bubble (the
narrator's text is kept whole). Bubbles fade in over 300 ms (smoothstep) and
out over 1.5 s; the narrator's waits up to a minute while its replies are
offered and closes in a second (half a second for a new narration) once one
is picked. In the original the box is 80 interface units wide, a
character's sits over the head (120 cm up) and the narrator's at 75 % of
the screen height.

The reply panel lists the player's lines: first the replies of a dialogue
the partner is holding with the player, then the topics of every dialogue
the two can hold; when both exist an `[Other]` entry switches between them.
A pick shows the player's line, then either takes that reply or starts the
topic; the panel closes when nothing can follow. A conversation waiting for
a reply ends after 30 s.

## Forms

The original's own forms and controls that Exarust rebuilds, with the
`Exanima.exe` 0.9.5 code and data they were read from:

- **Main menu** (`exa_game::menu_pages`): `TMainMenu`'s page table at
  `0x100122c40`.
- **Settings and Controls** (`exa_ui::settings`): `TSGSettingDlg`'s
  0x68-byte control records, the 9 tabs and buttons at `0x1002b2af0`, the
  27 Settings rows at `0x1002b2ea0` and the 22 Controls rows at
  `0x1002b1700`; the page switch at `0x1000a7e20`.
- **Control geometry** (`exa_ui::controls`): a list's extent at
  `0x1000fab10`, a control label's alignment at `0x1000fa110`.
- **Match board** (`exa_ui::match_board`): the 379-byte `0x100195c00` and
  the 121-byte `0x100195d80`.
- **Skill screen** (`exa_ui::skill`): `TSkillUI`'s 186-byte node placement
  at `0x100224570`.

When the GUI unit changes, every form rescales from the old unit to the
new (`exa_ui::gui_rescale`): lengths are divided by the old unit, then
multiplied by the new; a border of 4 pixels or more becomes its design
value times a quarter of the unit. On each axis a stretched form keeps
both edges' scaled gaps; others scale their size (or keep their frame
around a scaled client) and keep their right or bottom gap, their
centring, or their scaled place; a title-only form is its title and the
skin's base. Child forms follow inside the parent's client, then the
control records in their 16-bit fields: an odd width under 16 units and
an odd height grow by one, and a record's place is corrected by half its
height less four units before and after the scaling (one unit when it
is taller than eight). The common glyph sheet `GlphCommon` gives the
large atlas nineteen 64-pixel icons in a fixed order (the seventeenth,
at 0, 256, is the item views' marker). A message box is its text's width
plus 64 (at least 320) plus the skin's frame twice, 104 tall, centred
down the screen (or a quarter down), its top turned into design pixels
(over the unit, truncated, times 4); its
label sits at 12, 4, its Ok and Cancel buttons are 80 wide, 96 apart,
centred at grid row 13.

## Text boxes

Wrapped text (`exa_ui::text_layout`) and the multi-line text box of the
manual pages, character creation and item cards (`exa_ui::text_box`).
Text is Latin-1 bytes; line breaks are `\r\n`.

- **Breaks.** A line ends after a carriage return, after the last space
  that fits the width, or, in a line without a space, after the byte that
  overflows it (that byte stays on the line). A following line feed is
  skipped. A text under two bytes counts one line; one whose second-to-last
  byte is a carriage return counts one more.
- **Markup** (manual pages, tooltips): `[tcol=RRGGBB]` pushes a colour at
  the skin's tag alpha (eight digits carry their own; hex parsing stops
  at the first other byte), `[/tcol]` pops it; `[scrp=i]` slants glyphs by
  the main font's slant, any other `[scrp=…]` stops; `[algn=2]` moves a
  line right by half the width left at its last break; `[rimg=NAME]` draws
  a picture its size over 6 in units, centred in the frame, and moves what
  follows down by its height less a pitch. `{%` starts a span in
  (180, 200, 240), slanted, that ends after `%}` (the braces and percents
  draw too). A tag takes no width. The search for a tag's `]` stops at the
  text's second-to-last byte, so a tag closed by the last byte loses its
  last letter (and the `]` draws), as in the original. `[inptN]` is
  replaced by the name of the key bound to control N, title-cased with
  underscores as spaces.
- **Selection** is drawn behind the text, a pitch tall from a pixel below
  the line's top, in the skin's selection colour at half its alpha.
- **The caret** is the `|` glyph, half the font's gap left of the pen
  before the byte after it.
- **Layout.** The box wraps at its client width less a unit and its margin
  each side (200 units when it sizes itself), pitches lines at its spacing
  in units plus the font's line height, and counts lines as its text's
  height over the pitch (at least one). The content is the view plus the
  lines that do not fit; the scroll moves in whole lines (one line is
  1 / (lines − lines that fit)), snaps when set, and the drawn scroll eases
  a fifth of the way to its target each frame, snapping within 0.001. A
  box that grows takes the content height plus its insets.
- **Keys.** Typed bytes insert at the caret, replacing a selection. Enter
  inserts `\r\n`; Backspace and Delete remove a `\r\n` as one. Left over a
  `\r\n` passes it; Right from before one stops between its two bytes (the
  original's). Home and End go to the wrapped line's start and its end
  before the break. Up, Down, Page Up and Page Down (the lines that fit
  less two) keep the pen offset, landing on the nearer side of the byte
  under it, and scroll the target line in as the first line, or as the
  second-to-last when it is past the last visible one. Ctrl+Home and
  Ctrl+End jump to the ends and scroll there; Ctrl+C, Ctrl+X and Ctrl+V
  copy, cut and paste. Shift keeps the anchor. Tab goes to the owner when
  it takes Tab. A read-only box takes no keys. Enter over a selection
  sizes the text from its length before the cut, so bytes past the cut
  come back at the end (Exarust keeps the old storage there, guessing that
  the allocator resizes in place).
- **Pointer.** A press puts the caret before the first byte whose middle
  is right of the pointer (its line is y over the font's line height,
  whatever the spacing), past a line's end before its break. A drag moves
  the caret and scrolls the pointer's line in at most every 180 ms, also
  while the pointer rests. Stick directions go to the parent form, which
  scrolls the box a step on up and down.

Exarust's forms do not use text boxes yet; the box is ready for the
manual pages and character creation.

## Icons

The interface keeps two icon atlases (`exa_ui::icon_atlas`), one texture
each, `Icon_Manager`: a colour one for items and portraits (its resized
icons sharpened) and a single-channel one for the built-in glyphs, with
the level's cells. An icon goes in the first free cell; the first opens a
square the next power of two of the cell; a full atlas doubles its
shorter side, its height first, and a wider one repacks its cells in
order. A handle is the cell's index plus one; freeing a cell keeps its
pixels. Each change re-sends the whole image. Icons draw as one quad the
cell's size, upright, turned (eight recorded turns), mirrored, outlined
(the icon in the face colour, then four copies in the outline colour a
pixel left, right, below and above) or glowing (four copies two pixels out
in the glow colour at half alpha, then the icon in white). A colour icon
is hit by its alpha.

The twelve built-in glyphs (`exa_ui::gui_glyphs`) are plotted pixel by
pixel at the GUI unit into the small atlas, in this order: a cross (or the
first symbol of the pack's `GUISymbl`), right, left, up and down chevrons,
three window frames, minus and plus boxes, a drop-down chevron over a bar
and a tick. Strokes are one bright pixel with dimmer fringes; a fringe
drawn over a fringe halves it and adds its own. On a scale change both
atlases are emptied, take the new cells, and the glyphs are drawn again
(`GuiLevelState`, which also keeps the glyph atlas as a texture). Exarust
does not read `GUISymbl` yet and draws the cross.

## Tables

The table view (`exa_ui::table_view`) draws rows 6 units tall under a
header 6 units tall: cell text a unit in from each column's left and 11
units down for the first row (the glyphs' bottom edge), titles at 5
units (centred ones by their width), a header icon centred in its column.
It scrolls over all its rows but one; rows behind the header are not hit.
A press selects a row unless it is greyed; a second press edits an
editable cell (all of its text selected) or opens the column's choice
list under the cell; a double press activates. A press past the last row
of a table that grows adds a row and edits it, and committing an emptied
first cell of the last row deletes it. Enter commits, Escape abandons,
Delete on an empty edit deletes the row. Down passes greyed rows, Up does
not. The table never sorts.

## Item views

The item view (`exa_ui::item_view`) shows named entries with icons as a
grid (pitch the icon plus 4 units, rows 6 units more and a line, names
cut under the icon to one byte fewer than fit at the gap, with an
ellipsis, else centred), a list (one per row, the icon plus a unit) or
columns that fill top to bottom and scroll sideways (the icon plus 2 units
apart). Icons are 4 units unless the owner fixes them, and are rendered on
demand, one missing visible icon per request. A column entry is hit within
its icon and its name measured as bytes times the gap. A press selects, a
second renames (Enter keeps, Escape restores), a double press activates;
with check marks a press toggles an entry's mark (on the selected entry
the original sets every other flag bit too). Typed letters jump to the
next entry whose name starts with them, case-blind, the letters adding up
within 600 ms. Left and Right move by a line of entries outside the list
layout, Up and Down by one; F2 renames.

## Controls

The seven basic control classes (`exa_ui::gui_controls`), with their
default sizes: button 80 × 24, check box 16 × 16, image button 28 × 28,
trackbar 16 tall, label 20 tall, icon button and picture 28 × 28. Hover
lights a control that is not on; a release over it toggles it (a radio
group turns its others off and keeps one on; a three-state box goes off,
on, partly on). A check box shows the tick glyph centred while on. A
trackbar's knob is 4 units and 2 pixels with a 2-pixel margin (a marker
bar's is 5); its value is the pointer's offset past the margin and half
the knob over the travel, quantised to its steps (a stepped bar's knob is
its inner width over one more than the steps, at least 4 units); keys and
the stick step it, a continuous bar moving by a hundredth of a stick
axis (a float; the sum is taken in double precision). A picture is its
texture's size over 12, in units, centred.
