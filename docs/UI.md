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

### Text drawing

The text routines (`exa_ui::gui_text`) measure a run with the selected
font's advances and draw glyph quads from it: one line with its baseline at
`y`; centred in a width (moved by `round((width − total) · 0.5 + x)` when
narrower) and stopped after the glyph that passes the moved start plus the
width; an inclusive span; cut to `round(width) / gap` glyphs with the last
replaced by the ellipsis (0x85), the pen still moving by the replaced byte;
or the tail that fits, measured with the GUI level's main font, the first
drawn byte replaced by the ellipsis, placed where it would sit had the
whole line been drawn. Shortening a text with the ellipsis counts the
ellipsis and then the bytes before the last; at the k-th byte that passes
the width the first k − 1 bytes stay. A wrapped block breaks like the text
box, draws its glyphs hanging down from each line's top while the top is
within the height less a line, draws the lines it skips at the block's top,
and moves a centred line right by half its last break's spare width. The
line count of an empty text is zero. The width fit narrows a width to the
average line width, widens it by the GUI unit until the text takes no more
lines than before (a final carriage return counting one more), and keeps
the widest line of that pass; a line's width there counts every byte that
fits, the head of a wrapped word included.

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
| standing | the ring | the arrowhead over a point within reach (`docs/PLAYER.md`, The reach), else the cross |
| a press of the left button refused (`docs/PLAYER.md`, The hand), until its release | — | the cross |
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
- the reach is the hand's (`exa_game::hand`): the pointer's ray at the
  level, the props and the characters, the hit at most 245 cm high in the
  world, and its offset from the player's hip within 300 cm up or down and
  300 cm across;
- the chevrons show while the right button walks to the cursor, in combat
  mode too;
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
- the focus bar shows the player's caster's focus and the capacity it
  regenerates to ([THAUMATURGY.md](THAUMATURGY.md), Settle), 1 and 1 for a
  body that casts nothing. Exarust has no shields: the stamina gauge
  never shows;
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

The logic is in `exa_script` (`bubbles`, `dialogue`); `exa_ui::speech`
turns a frame's bubbles into batches and `exa_ui::dialogue_choices` does the
same for the reply list, but no form is painted by the running app yet.
A spoken line shows for 1000 ms + 50 ms per byte, the narrator's under 10 s
stretched by (2 − d/10 000), the player's halved. A speaker's new line closes
its old bubble (fading four times as fast) and waits 375 ms past its end; a
character's line is cut at a CR and the rest follows as another bubble (the
narrator's text is kept whole). Bubbles fade in over 300 ms (smoothstep) and
out over 1.5 s; the narrator's waits up to a minute while its replies are
offered and closes in a second (half a second for a new narration) once one
is picked. A line is wrapped at 80 interface units (320 pixels), narrowed
to its widest wrapped line, and is as tall as its line count times the
font's line height less two. A character's box sits over the head (120 cm
up, taken to camera space by the rigid inverse view and projected), the
narrator's is centred at 75 % of the screen height, and both keep four
units from the screen's edges. The box is a rounded fill of radius three
at alpha 128 times the fade; the narrator's is grown by four units on each
side. The white text (alpha 160 times the fade) starts four units below
the box's top less the glyph height and half a unit; every box is drawn
before any text.

The reply panel lists the player's lines: first the replies of a dialogue
the partner is holding with the player, then the topics of every dialogue
the two can hold; when both exist an `[Other]` entry switches between them.
A pick shows the player's line, then either takes that reply or starts the
topic; the panel closes when nothing can follow. A conversation waiting for
a reply ends after 30 s.

## Arena match board

The arena's "Matches" form has four tabs: standard matches, the daily
events, the weekly tournament and the season. Its code is
`exa_ui::match_board`; `exa_game::arena_board` fills it from a career and a
generated daily board. The rules are plain functions and values and need no
Bevy app. Evidence: the original's board constructor, tab switch, list
refreshes, fighter assignment and coin cell callback, read against their
assembly, and the standard-match and rank-label tables in `Exanima.exe`.

- **Selected match.** Every tab copies one match record and the lineup is
  built for it: its lowest rank is the rank floor, its side size caps the
  lineup, its kind picks the Captain rule, flag 1 lifts the one-leader and
  marked-fighter rules, and flag 4 names a weapon for a tournament event.
  The record carries the arena's fields: kind, flags, rank window, day slot,
  rolled rank, fee, prize, side size and weapon row.
- **Standard tab.** Three lists stand side by side: the six standard
  matches, the six ranks (Inept, Aspirant, Novice, Adept, Expert, Master)
  and a tier list. The standard matches are Duel (one a side, 3 copper,
  1 silver), Doubles (two, 6 copper, 4 silver), Skirmish (three, 9 copper,
  6 silver), Fray (one, 3 copper, 2 silver), Reserve (three, flag 1,
  3 copper, 4 silver) and Pugilism (one, unranked, 1 copper, 6 copper). A
  rank change moves the tier to the same row; the tier cannot exceed the
  rank and the tier rows above the rank are greyed. The refresh stores the
  rank as the floor and the tier (none for Pugilism), and multiplies the fee
  and prize by the rank plus one: the coins are unpacked at twelve copper
  to the silver and twelve silver to the gold, multiplied in 32 bits and
  repacked with a 16-bit gold count and wrapping silver and copper bytes.
- **Message box.** The standard and events refreshes write "There are no
  more matches running this day. End the day to participate in more
  matches." once three matches were entered that day, and clear it
  otherwise; the tournament refresh clears it.
- **Daily events.** Six events, five columns: match, rank (the day slot),
  tier (the rolled rank), entry and prize. Challenger (kind 8) shows a dash
  in the tier column and an unranked event shows Inept. A rank index past
  Master reads on in the original's label table: the dash, Merchant,
  Trainer, Physician and "+". Entry and prize are packed coin dwords drawn
  as coin icons, or as a dash four pixels right when the value is zero. An
  event with flag 1 is greyed; selecting one moves the row to the first
  event without it, and when all six have it the row moves to the last one
  while the selected record stays.
- **Tournament.** Six winner rows (event, tier, winner). A winner of 1 is
  the player's company, below 1 a dash, and from 2 an entry of the
  tournament's company table, which holds a company id less one. The status
  line says "Tournament begins tomorrow." or "Tournament begins in N days."
  off the tournament day; on it, "Tournament is in progress." with no
  entries, "The tournament has ended." when every entered event is settled
  and the last event has a winner, else "No more events entered.". With an
  entered event that has no winner, the primary button reads "Begin" (or
  "Continue" once the first event has a winner) and a second button
  appears. A weapon event shows the reward panel with the weapon row's
  name, shortens the lineup to 120 pixels and marks the panel when the
  first fighter entered in the event's tier does not carry one of the row's
  up to three weapons (the first zero ends the list). On a tournament day
  with entries the tab is forced to the tournament (the show hides the tab
  it leaves) and fighter assignment is locked. Leaving the tab restores the
  primary button and its "Enter" label.
- **Season.** Four weekly winner lines. A week already played with no
  winner says "No winner"; a zero winner or one above 20 is a dash. The
  season is four weeks, the season index is `((day + 6) / 28) mod 4`, and
  the reward picture is `seasonrwrd{a..d}.rfi`. The four reward items are
  shown in the order first, third, second, fourth. The completed weeks are
  `ceil((day mod 28) / 7)` up to day 21 of the block, else zero.
- **Fighters.** The left table lists the company's available fighters and
  the right the lineup; both show name, rank and health. A fighter's rank is
  the sum of its six skill levels (each skill word over 5000) halved and
  capped at Master. The rebuild walks the lineup order: a fighter joins when
  its rank reaches the floor (on a Captain event, after the first fighter,
  two ranks lower), at most one leader joins unless flag 1 is set (the
  player counts as a leader), and it stops at the side size. A double click
  joins when there is room and otherwise benches the last teammate to the
  end of the company table; a drop swaps with the row. A marked fighter
  swaps with the first marked teammate. On a Captain event with more than
  two teammates of whom fewer than two reach the floor, a fighter under the
  floor may not replace one who does, and a qualified teammate is moved
  first. The fighter's place in the lineup order becomes the row it joined,
  or the first place after a marked swap or a bench. A fighter moved back is
  inserted in pool-slot order and keeps its place in the lineup order.
- **Day and confirmation.** The "Entered n/m" label writes one character
  per count, adding 0x30, so the original shows a single digit (and
  punctuation past nine). After each rebuild or move the primary button is
  disabled once three matches were entered or while the lineup is short,
  and hidden on the season tab. A day-end control is enabled once a match
  was entered (on a tournament day the arena's own checks decide).
- **Layout.** The form is 604 by 656 (604 by 636 in the compact form). The
  match list is 196 wide at x 0, the rank list 192 at x 200 and the tier
  list 192 at x 396, all from y 56 and 180 tall. The event table is 588 by
  180 with columns 140, 100, 100, 114 and 106; the winners table 392 by 180
  with columns 120, 100 and 120. The fighter tables are 292 wide at x 0 and
  x 296 from y 348, 244 tall (224 compact), with columns 140, 68 and 132.
  The message box is 520 by 96 at 40, 276. The tournament picture is 192 by
  180 at 396, 56 and the reward panel 292 by 120 (100 compact) at 296, 472.
  The season table is 280 by 132 at 32, 60, one 160-wide column.
- **Drawing order.** The form, the four tabs, then the visible child views
  in the order the constructor made them: match, rank and tier lists,
  picture, reward panel, events, winners, message, company, lineup and
  season tables. A table draws its frame, its column headers, then its rows
  left to right. The rows are 30 pixels tall (a guess from the six-row and
  eight-row fits). A point maps to a tab, the primary button or a visible
  cell (`Board::hit_test`), and a click selects the tab, match, rank, tier,
  event or winner or confirms the board (`Board::click`).

Known guesses: the row height, the tab strip's and the template controls'
rectangles and labels, the prize item's slot, where the template's controls
draw relative to the child views, the class bytes for the three staff
professions, the meaning of flag 1 on a daily event, and the day-end role
of its control. The weapon check's item kind of a fighter's event value and
the tournament day's day-end checks are the arena's and are supplied by the
caller. The board does not count matches; the arena does.

## Arena company windows

The hub's company windows are modelled as rules (`exa_game::manage`) and
drawn through the GUI path from the hub (`exa_game::hub_windows`): the match
board, Manage Company and the trade window open from the hub's click hook on
a `GuiTask`, and their board paint list, roster rows and stock are painted
there. Their controls are read from the original's form tables. The roster
cells, manage refresh and `arsenalico.rfi` icon sheet are in
`exa_game::manage` and `exa_game::manage_icons`.

- **Manage Company** (0x2c4 × 0x1d4): Roster and Report tables, Manage,
  Arsenal, Recruit and Hire, the Owned, Upkeep, Earned today and Spent today
  coins, End Day, Close and Records. The roster's columns are Name (0x84),
  Rank (0x50) and Health (0x84). A rank cell reads Merchant, Trainer or
  Physician for a specialist, else the rank name, marked "+" while the
  fighter is ready to train. A name is drawn in 255, 210, 100 at three
  fifths of the skin's alpha for the manager, 220, 180, 160 at two thirds
  for a specialist, 160, 180, 240 at two thirds for a hired combatant, and
  in the skin's text colour for a recruit. Recruit closes at eight fighters
  and Hire at ten employees. A row dragged onto another takes its place
  (`staff::Staff::move_employee`). A report line opens the board's
  tournament tab, the hire list on the first candidate of its profession,
  or the roster on its first row.
- **Arsenal** form (TLoadoutForm): a list with Previous, Next and Dismiss,
  the Rank slider, "Points: n / m", Encumbrance, five shelf and five box
  buttons, CLOSE, and on the hire page HIRE with the hire fee and upkeep.
  Its pages are the roster, the hire list (opening on tab 5), a
  specialist's record (shelves filtered by 0x200, no rank controls, no
  tab) and an event's lineup; the others filter by 0x400. Previous and
  Dismiss are disabled on the first row and Next on the last; a step loads
  its record and is committed after the frame; Dismiss asks "Are you sure
  you want to dismiss " the name "?" and moves the selection up. The
  slider's rank is its value × 5 rounded to even; the points line halves
  the cost plus one and the doubled loadout limit. A record's title is
  "Name (Rank)" or "Name (Profession)"; a candidate shows "Profession: …"
  and the profession's description. A pressed shelf or box button stores
  the view in its box and loads the new one, sorting a marked box once. An
  event's lineup opens on its first entrant at the event's rank tab, and
  closing it writes the hands back with the second hand's item cleared.
- **Company Status**: the company's name, Renown and Reputation "Unknown",
  Morale "Good", Total Staff (the manager counted) and Masters (fighters
  above Expert, or "None"). **Company Records** lists the thirteen company
  records, the last two as coins.

## The running hub

`exa_game::arena_hub_live` enters the hub once when a play level named
`arenahub` is loaded: it calls `arena_hub::enter_hub` with the hub's roll and
the flow's alternative-music setting, plays the drawn track through the audio
crate's level-music resources, installs the click and session hooks, and
places the manager by `arena_spawn::manager_plan` on the hub's known start
(0, floor + 5, 300 cm). Each frame `arena_hub::hub_fall` checks a manager who
is dead while the session plays: the answer ends the session, and a health
under 0.01 wipes the arena slot through `slots::Slots::drops_after_death`
when the host supplied the slot (`HubSlots`).

A left click that is not a drag reaches the hook through `hand::WorldClick`.
The hook reads the object's name and side, or the character's condition and
merchant profession, asks `hub_click`, and opens the answered window on
`hub_windows`: the board from `arena_board::open`, Manage Company from
`manage_refresh`, and the trade window from `shop_window` with the shop's or
the merchant's stock. A board pick of Enter, Begin or Continue plans the
match (`arena_match::start_match`, `match_layout::layout`, `generic_plan`,
`hire_plan`, `beast_plan`, `player_participant`, `pick_weapon_slot`) and the
launcher changes to the drawn arena in process. Escape and a
right-button release close an open window. Opening the board or Manage
Company makes the hub busy (the board's open and close set and clear the flag
in the original); the trade window leaves it alone.

The notice boards and the shop counter are static scenery with no body; their
colliders remember the scene node they were built under
(`exa_physics::ColliderNodes`), so a click on `\xaaArenaShop` or
`\xaaArenaMach` reaches the hook. The ArenaMach side value is a float the
original reads from the clicked node; which field it is is not traced, and
the node's world x stands in for it.

Not drawn: the board's pictures, prize items and reward slots, the manage
window's controls and report lines, the trade window's item icons and
character preview, and the windows' own close controls.

## The running arena match

A board pick hands the career and the match's setup to a restarted process
(`arena_handoff`) that loads the drawn arena. `arena_live` stands the round's
bodies: the play start's player, the company's other entrants, and each
opponent built from its own generated record — the random NPC pass and the
arena outfit tables from the object database — through the merged character
mesh; a tournament side takes its company's forced outfit profile and the
event's drawn weapon. The Beast, the company entrants' and the hub
specialists' records are not kept, so those bodies still take actor
templates [guess].

A round's result opens the end screen as the original builds it: screen 3
shows the single `NEXT FIGHT` row and screen 2 the single `EXIT;` row (the
arena mode's `& 2` test leaves the match-end menu one row), both at scale
0.9. `exa_game::arena_live::ArenaEndScreen` paints it over the game through
the GUI path and takes the row on Enter or on a pointer release over it; Up
and Down move the selection. The first row is the arena Continue: a dead
manager goes to the main menu, a next round stands the winner and the
reserve or a fresh opponent after 13 ms, a tournament's next round resolves
its bracket first (the won group keeps team one, every other group draws a
winner with the arena's roll) and stands every surviving side on the ring,
and the match's end settles the result and hands the career back to the
hub's process with the 300 cm, half turn, `0x100` placement. Escape opens the
arena pause page (`CONTINUE`, `SETTINGS`, `FORFEIT`, or `EXIT` once the
session was won or ended); its FORFEIT settles the match as forfeited and
returns to the hub, and a dead manager's forfeit is ignored. The score
board, the round banner and the per-fighter condition bars are not drawn.

## Forms

The original's own forms and controls that Exarust rebuilds, with the
`Exanima.exe` 0.9.5 code and data they were read from:

- **Main menu** (`exa_game::menu_pages`): `TMainMenu`'s page table.
- **Settings and Controls** (`exa_ui::settings`): `TSGSettingDlg`'s
  0x68-byte control records, the template tables of its 9 tabs and
  buttons, its 27 Settings rows and its 22 Controls rows, and its page
  switch.
- **Control geometry** (`exa_ui::controls`): the list-extent and
  control-label alignment routines.
- **Match board** (`exa_ui::match_board`): the board's constructor, tab
  switch, list refreshes and fighter assignment.
- **Skill screen** (`exa_ui::skill`): `TSkillUI`'s 186-byte node placement
  routine; the requirements, states, ring icons, tooltips and
  forget dialog below.
- **Character creation** (`exa_game::create`, `exa_ui::creation`): the
  appearance page's NAME/HAIR/NEXT, the equipment page's APPAREL/WEAPONS
  tabs, or EQUIPMENT/LOBBY when the versus lobby script is installed, and
  its RECRUIT/BEGIN/READY/CREATE button, the continue screen's save or
  checkpoint button, the skills page's SKILLS, Current skill progress,
  Global experience and New global experience captions, and the icon sheet
  of four 96-pixel cells scaled to eight GUI units. The apparel panel sits
  at 4, 24, 172 wide and 620 or 568 tall; the weapon panel against the
  parent's right edge less 176; the skills panel 216 wide, 516 or 660
  tall, 12 in from the right; the skill tab slides four units per index
  and the panel narrows by the same; on the continue screen the character
  view starts at 59 units, the item list at 4, 116 (228 by 180), the
  details form at 0, 328 (236 wide, 320 or 260 tall) and the name inside
  it at 4, 40, 228 wide to its bottom. The continue toggle is enabled only
  while both a save and a checkpoint exist. The form's opening (the GUI
  clear, the removed task and three input actions) and the equipment page's
  back switch (the first built of the weapons, skills and lobby forms
  hidden, the skills panel's 0x2d slide when it is that one, the versus
  forms hidden, a non-continued character's appearance panels shown again
  with both tab words at 0x110, the NEXT button enabled) are modelled,
  as are the preview's icon list, its material pass, the dummy-head
  renderer (`dummyhead.rfc`, the `0x00a0a040` options, one added reference
  and the three hooks), the continue page's character view (model polling,
  item passes, the fifteen-byte name and the CONTINUE/VERSUS title), the
  power tree's two creator callbacks, the name warning and the reset and
  delete dialogs. The rules (gear
  filter, underwear pass, starting skills, experience budget, kit tables,
  page flow, save choice) are plain functions in the game crate with tests
  pinned to concrete numbers. The form itself, its model preview, sound
  playback and the painted power tree are not drawn yet.
- **Technique selector** (`exa_ui::technique_selector`,
  `exa_game::techniques`): one skill's eight techniques, 332 × 112
  pixels, editing the record's six skills (an experience word and five
  technique bytes each, a level 5 000). A pick into an empty slot replaces
  the technique in training (the first skill whose slot past its level is
  filled) and gains the unspent experience up to a level, or `pool · 4 /
  (4 + Σ levels)` capped at a level under the weighted rule, which draws
  `gain · divisor / 4`; a forget closes the slots up, drops the techniques
  that name the forgotten one (the character data's prerequisite pairs)
  and a level for each, then refunds into the technique in training. The
  name sits at 16 units, the description at 18, the 47-byte confirmation
  at 12; a key goes to the base only for a pick in the confirm mode or the
  hovered code 9, else closes with 0x40.
- **Compact skill screen** (`exa_ui::compact_skill`): character creation's
  short tree, 240 × 800 at (400, 50); rows 30 units apart on a four-skill
  tree, else 22 (wide) or 24; a margin of 4 units, plus 6 for a title, plus
  5 for the compact flag on a narrow interface; the hover word `row << 8 |
  column`; a hit strictly within five units (a later row wins); the wheel
  takes the nearest row (horizontal) or the row above or below (vertical).
- **Arena loadout** (`exa_ui::loadout`, `exa_game::loadout`): the 1024 ×
  768 editor of one fighter's kit at one rank. The arsenal is 228 wide at
  (4, 180) and `492 · opacity` tall (the opacity 0.83739835 on a wide
  interface), the equipment 236 wide at the parent's right edge, y 88. A
  weapon costs its points less what it displaces from the hands; an item
  is equipped when a copy is free, its rank is at most the form's (5 when
  unset) and the cost fits the limit. A rank's kit is the record's own
  loadout; the editor's lists of it are Exarust's model (guess).
- **Divination cards** (`exa_ui::div_card`): three distinct cards out of
  the twenty-two on `divcardsa1.rfi` (a 6 × 4 sheet, first card at cell
  2), the screen darkened to 128 over a second, dealt over 600 ms each
  and turned over 900 ms each by the ease `(3 − 2t)·t²`, a half turn about
  an axis swinging from (0.3, 0.9, 0.2) to (0, 1, 0), held from 4.5 s,
  each card over a drop shadow with a four-unit fading band.
- **Company emblem** (`exa_ui::emblem_form`, `exa_game::emblem`): the
  background (1…50) and design (1…80) layers, the two mirror bits, four
  palette indices (0xd3, 0x72, 0x5a, 0x4e on a new form) and the
  randomiser (`state · 0x8088405 + 1`, each draw the high half of `state ·
  n`, four records of history), previewed by the four-texture shader.
- **In-game screens** (`exa_ui::game_gui`, `exa_game::gui_screens`): the
  inventory, loot window and journal show rules, the eight-slot open
  container registry, the journal's entries, the power-slot and counter
  controls, the HUD's marker, alert, curtain and compass and the close and
  save scopes; the running app opens the inventory and journal from the I
  and J hotkeys. The form base class's layers, fade, scroll, hit, Tab and
  frame-code rules are in `exa_ui::form`.
- **Destination screen** (`exa_ui::selection`, `exa_game::destination`):
  the teleporter's four (or one) rows on a ring of 40 units, node `i` at
  `(1 − turn) + (2.5 − i)·2π/(count − 1)` with the engine's sine across and
  cosine down; a 500 ms reveal (the alpha eased once, the ring twice), a
  256 ms close, a 200 ms hover flash and the 14-unit hit on the three
  orbit nodes; only the adapter's orbit wobbles. The row fill is the
  original's: the shipped 7, 10, 12, 13 show as 13, 7, 10, 12. A click
  closes the screen and starts the journey: the bubble 98 cm above the
  teleporter, the teleporter's glow (1.5 s up and 7 s held for the
  adapter, 1 s and 1.4 s otherwise) and a small crystal charging a point
  per 12 ms. The level changes when the resting bubble is told to shrink;
  the adapter's held bubble instead marks the teleporter single-row and
  saves while the player's red health is above 0.01.

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

### The game flow

`exa_game::flow` is the original's game-main flow as a state machine over a
mode word, a phase word and a session hook. Its methods return named steps
(a menu page, a level load, a save, a camera shot, a cursor warp) instead of
calling the rest of the game, so the order is testable.

- **Boot.** First-run settings, the actor library, the game, item and skill
  interfaces, the hardware settings, the GUI scale, the menu key task, the
  first-run tip, the unlock words, the experience system, the two save
  managers (story signature, arena signature and data version), the
  transition form, the arena data, the audio, the splash, then the title
  page. The script hooks register at the unit's initialisation, before the
  window.
- **Modes and phases.** The mode word is 0 (title), 2 (practice), 4 (story),
  8 (arena) or a scripted mode (versus is 0x1010). Phases: 0 transition, 1
  title, 2 the character creator, 3 playing, 4 won, 5 ended, 8 round won,
  0x10 forfeit, 0x20 paused (the arena's pause screen) and 0x40 the class
  select. A switch clears the reconstruction state, stores the outgoing
  mode's player, restores the incoming one and sets the incoming hook: the
  story's for story, none for the arena, the title and a scripted mode
  (whose enter hook runs); practice keeps its hook until its start installs
  one. Story mode uses checkpoints, arena mode overrides the inventory (the
  flag words' reading is a guess).
- **Starts.** A story starts at `exanima01` with its background's roles (7
  and 1, then the background's own, the list cleared first), flags, pose
  and wake-up motion; the unknown background carries item 0x10c and gets
  the thaumaturge's power word 0x510000 and 4.0, the knight and proctor hold
  item 0x1f in the off hand and carry it (flags 0x100).
  The villager starts with condition and animator flags 0x600 and turn
  0.45, the knight and proctor turn 1.0.
  The unknown background and the villager get a starting light (turned −48°,
  110°, 88° and 8°, −150°, 50°), the unknown one Derrin. The spawn turns are
  the engine's angle builder's rows (`(cz·cy, sz, −cz·sy)`, …). Practice
  starts at `arena`/`arenae` with `opponent01`/`opponent09`, role 0x10, the
  ladder's music (0.1 and 0.25; both drum-and-bass tracks at 0.15) and the
  player in the ring's middle at its standing height plus 5 cm; the next
  fight stands it at its measured stand height. A continue loads the save's
  locale (or `exanima01`); the arena continue loads `arenahub` with the
  hub's music.
- **Camera shots.** A point shot puts the camera at the point plus an
  offset: (700, 1180, 1100) cm for every shot of the group (practice start,
  next fight, story and arena continues), (300, 1180, 1300) for the arena
  unit's far shot; a fast shot forces 1300 on the view. The story start puts
  the camera 600, 600, 1300 cm off the player. The cursor warps to the client
  centre plus (120, −200) (or (−200, 0) for the story) × height/1140.
- **Transitions.** Everything that changes the world runs behind the
  transition screen, 100 ms later: the quit to the title, the story respawn
  at `exanima05`'s portal (15 cm up, the camera left alone, the save written
  first), the revival (also saving first), a level load at the exit's spawn
  point. The phase before the transition is kept.
- **The frame.** While no menu covers the game and no transition runs, each
  frame ticks the dialogue engine and then the mode's hook. The story hook
  checks the three wake-up flags, runs discovery once a second and handles a
  fallen player (dead with at least 0.01 health: knocked out for five
  seconds, then revived; otherwise dead). The get-up writes the animator's
  held pose (flags 0x600, blend 1, move and turn 0.49) until five hundred
  milliseconds after its start, then its clocked pose (flags 0x20, blend
  0.4, turn 0.4, the clip's own value less 1490 ms); the revival sets the
  wake bit and pushes the clip clock two seconds on. The practice hook waits
  a second; then a dead player dies, or the first dead opponent ends the
  round; the eighth fight wins the ladder and unlocks expert. F1 in a
  session stops the dialogue and the hook behind the in-game page, as the
  original's frame task does.
- **Death and the end menu.** Death shows the end menu and, in the story,
  drops the save unless the player can be rebuilt (then the first death
  strips them). The end menus are EXIT; (a won story or arena), RESTART/EXIT
  (practice over), CONTINUE/EXIT (rebuilt), CONTINUE/LOBBY/EXIT (arena) and
  NEXT FIGHT, at scales 0.9, 0.85 and 0.78; the arena's refreshes the pad
  bindings and starts with no row selected (a session's begin and continue
  clear that flag). Its confirm key plays the menu's click at
  0.06 with a pitch of 0.8 to 1.2. `exa_game::end_menu_screen` draws the
  menu over the game through the GUI path, the selected row through the
  menu's text program, and runs the row on Enter or on the release over it:
  the next practice fight, the story's respawn or end-menu exit, a practice
  restart (back to the menu form), the arena's continue (the live match's
  own end screen), and the exit. The centered row placement is
  Exarust's (the original's form class is outside the game-main unit). The
  arena's pause page reads its last row EXIT once the session was won or
  has ended, FORFEIT before.
- **The menus' rows.** The title's CONTINUE reads the story's save manager,
  the start page's the arena's; each grey out only when its own manager has
  no save. The in-game page's first row is NEW GAME in a mode whose word
  has the value-2 bit (practice) and CONTINUE otherwise, and the pause
  pages never grey CONTINUE out. The row origin is 0.38 (title), 0.45
  (start, class select) or 0.39 (the in-game and pause pages).
- **Leaving.** The story's end-menu EXIT closes the game view and the
  dialogue, clears the story bit of a stripped player and saves without the
  death rule (any other story saves with it), then leaves to the title
  behind the transition. Leaving a session stops the hook and the frame
  task, closes the dialogue, the prelude and the save package and hides the
  end menu. Back to the title shows the menu form's title logo again, plays
  its theme and builds the title page. A level load's
  finish closes the end menu and the prelude and returns the session to
  playing.
- **Level exits.** A level the locale table names is left behind the
  transition; a player with neither dead bit loses the controls. Every exit
  saves (without the death rule); levels below 7 also keep a checkpoint when
  `CheckPoints` is on (level 6 always) and the save holds neither
  `exanima04` nor the level. An exit whose fade bit is set also fades the
  screen out and hands the player's body to the AI before the transition.
  The locale's ambient music is the host's.
- **The quit autosave** is the application's quit callback: in the paused
  phase the arena unit's pause callback runs; while playing, a story saves
  with the death rule, an arena saves from its in-game page or forfeits from
  its pause page, practice writes nothing, and the save manager's package is
  closed. It does not return to the title.
- **The exit-trigger nudge** pushes a continued story's player out of an
  exit trigger that holds it: the squared distance to the centre within
  (radius + 50)², the squared box distance within 50 and an exit-marker
  object. The offset, its height zeroed, is turned into the trigger's frame
  by its rows; the axis with the smaller clearance is taken and the player
  moves along that row by `(extent − local) + 75`, negated when the local
  offset is not positive (the original's arithmetic, which overshoots on the
  negative side).
- **Wiring.** `FlowPlugin` boots the flow, ticks it on the 12 ms game clock,
  watches a play level's player (a sandbox practice-mode session without
  the practice round hook, since a launched level is not a practice arena
  and its corpses would end a round at once) and turns a
  teleporter's `TravelToLevel` into a level exit; F1 then opens the in-game
  page with the flow's mode and phase words and hides the session hook while
  the page is up. `FlowSteps` collects every step and the app applies the
  menu, end-menu, title-logo, game-view, controls, health and music ones.
  The launcher sets `BootToMenu(false)`, so the boot builds the flow's title
  state without covering the level or the splash; the splash path shows the
  title at its hand-over.
  The world, save, camera, character and transition-screen steps are left
  to the host; a level load's request (a practice start on a fresh title,
  the next fight, a teleporter, the story respawn) becomes an in-process
  level change (`exa_world::level_lifecycle`), and the player is placed
  across it. Its app-owned first steps (the menu form hides, the end menu
  closes) run before the change.
  The end menu's rows follow the end-session dispatch, which branches on
  the mode first: practice's first row is RESTART (menu 2: the prelude
  closes and the session leaves to the menu form behind the transition)
  or NEXT FIGHT, its second row returns to the title behind the
  transition; the story's first row is CONTINUE on the reconstructed menu
  and otherwise, like its second, the story exit; the arena's first row is
  the arena unit's continue (the live match's end screen) and its second
  (LOBBY) the return to the title; the arena's third row and a scripted
  mode's rows do nothing there.

### The splash screen

`exa_game::splash` is the start-up splash. Each tick draws a black backdrop
over the client at the backdrop's alpha, then runs its state: a one-second
renderer warm-up; `bmelogol01.rfi` fading in from black over 600 ms (its grey
level rises, the backdrop stays opaque) and holding to two seconds;
`bmelogol02.rfi` with `BME Critters` at 0.2, a 200 ms hold while the warm-up
goes on, a 300 ms fade to black; the rest of the warm-up; the hand-over (the
title page is built behind the splash); a one-second wait; the menu theme;
and a one-second fade of the backdrop that uncovers the menu (closing under
the float 1.1920899822825959e-07). Logos are centred at
`round(H/135)·0.125`. Fades are computed in double and kept in single; the
grey level and the alpha are rounded half to even and kept as a byte. Every
comparison uses the frame's time and every stamp the system clock, so an
elapsed time is their signed difference. The splash also drives the
transition audio gain (silent until the second logo, silent again after the
menu wait, rising with the last fade) and the marker outline fade. The form's
hit-test and destroy order are `exa_ui::menu`'s. `exarust --splash` shows it
at start-up (about five seconds) and opens the main menu at the hand-over;
without the flag nothing starts it.

### The main menu screen

The running app opens the main menu with F1 (`exa_game::menu_screen`); the
original opens it from its session flow, which belongs to the game modes.
F1 shows the flow's page: the in-game page while a session runs, the title
otherwise, with the flow's mode, phase and page state. The page is painted
through the GUI draw path (`exa_ui::gui_draw`): a near-black backdrop, the
title logo (Exarust's wordmark in the base font until the logo art is
wired), and the rows from the menu font's sheet with the unit's own text
program on the selected row. The screen follows `TMainMenu`'s rules
(`exa_game::menu_pages`):

- **Pages.** Title: BEGIN, CONTINUE (greyed without a story save), ARENA,
  MANUAL, SETTINGS, EXIT. Arena: NEW CAREER, CONTINUE (greyed without an
  arena save), PRACTICE, VERSUS (only with the versus lobby), BACK.
  Practice: NOVICE, EXPERT (greyed until it is unlocked), BACK. In-game:
  NEW GAME in a mode with bit 2 (practice) and CONTINUE otherwise, then
  MANUAL, SETTINGS, EXIT; in a session this page is the pause too. The
  arena's own pause page is CONTINUE, SETTINGS, FORFEIT (EXIT once the
  session was won or has ended); the story's pause page is the same shape
  with EXIT, defined by the builder but never built by the executable. The
  row origins are 0.38, 0.42 with five arena rows (0.45 with four), 0.45
  and 0.39 of the height.
- **Geometry.** The row scale is `round(H / 135) · 0.125`. The page's top is
  `round(H · origin)` and a row advances `round(70 · scale)` (single
  precision, half to even). A painted row's baseline is the top plus the
  menu font's ascent; its x is `(W − width) >> 1` as an unsigned 64-bit
  shift. The hit band of a row starts at the top, is the ascent less the
  font's second origin field tall, spans `round((W − width) · 0.5)` plus
  its width with strict bounds, and the next band starts
  `round(70 · scale + top)`; a disabled row takes no point but advances.
  While a manual opened from the menu covers it, every point is kept;
  otherwise a point on no row is kept or let through by the menu's flag.
- **Paint.** Every row but the selected one is drawn in order, white or, when
  disabled, grey at half alpha; the selected row is drawn last in white with
  the menu's text shader, a third of a unit lower while it is pressed. A row
  with marks gets `[` or `[[` appended. With no selection no row is singled
  out. The menu's text shader is the game-main unit's own small program
  (`exa_game::menu_text`): it samples the glyph, mixes a tint from the
  glyph's red channel (`(1, 0.75, 0.4)` to white), gains it 1.1, tone-maps
  by `rgb/(0.5 + rgb)·1.5` and multiplies the vertex colour. The draw path's
  `SkinMode::MenuText` runs it over the row quads; a screen whose menus font
  texture is not installed falls back to the base font through the same
  path.
- **Selection and click.** The pointer selects the row under it and Up and
  Down move the selection. A press marks the pressed row; its command runs
  on the release over it, or at once on Enter. The commands, by page:
  title BEGIN (a story game at the class select), CONTINUE (the story
  saves' character screen), ARENA (the arena page), MANUAL, SETTINGS, EXIT
  (quit); arena NEW CAREER (an arena game at the emblem generator),
  CONTINUE (the arena saves), PRACTICE (the practice page), VERSUS (the
  versus lobby's creator) or BACK; practice NOVICE or EXPERT (a practice
  game from the creator) and BACK; in-game CONTINUE (resume) or NEW GAME
  (a new practice character), MANUAL, SETTINGS and EXIT (arena: save and
  go to the title; story: the story's quit; otherwise the title); the end
  pages resume, open the settings, or finish the match, as a forfeit
  unless the session was won. Escape goes back from the arena and practice
  pages, as the original's does at the main menu, and closes the screen
  elsewhere; Backspace goes back.
- **Font.** The original draws the rows with its menu font record, whose
  texture is `titlefont.rfi`: 112-pixel letter cells, A to Z in four
  columns, ascent 98, a space of 36, line height 128 and a trim of 6,
  rescaled by half the row scale (every metric and the advances of `A`
  through `a` rounded half to even, when the scale moved by 0.001 or more).
  A row's width is its advances (a space as the gap) less the trim. The
  screen measures and places the rows with that record but draws them in
  the base font until the sheet's quads are wired.

The session and form work behind the commands (starting a game, the
character screen, the manual, the settings form) belongs to the game modes;
the screen performs its own page changes, the resume and the quit at once
and sends the rest through `MenuChoice`. `FlowPlugin` applies those the
running game has: the resume, a practice start or restart, the story start
(Exarust's unknown background), the story end-menu exit and the leave to
the title, and it applies the flow's menu, end-menu, title-logo, interface,
controls, health and music steps, and a level load or teardown becomes an
in-process level change. The character screen, the creator, the manual, the
settings form and the live save and export steps are not wired.

### The window manager

`exa_ui::gui_core` is the original's GUI main unit, apart from the form
classes themselves: a `FormHost` supplies the virtual methods it calls (hit
test, select, press, release, drag move, drop, close) and `GuiTask` keeps
the task's state. A control value of `i32::MIN` means none.

- **Focus.** Only the modal form (or no form) may take the focus while one
  is set; another form clears it. A new form deselects the old one and
  starts with no control; a control the form has not selected is selected
  and stored, otherwise the stored control stays; the control -1 adds the
  form's own response. A select answer with 0x100 marks a control with a
  tooltip: the focus then requests it, stamped now, or 400 earlier when a
  tooltip already shows and the answer lacks 0x200. In gamepad mode a
  control whose state has 0x400 drops the focus when its form is left, and
  one whose style has 0x800000 takes its form's focus.
- **Tooltip.** The task's tick shows a requested tooltip once 400 clock
  units have passed (an unsigned comparison), only if the focus is still on
  that form and control; the control's record picks its class and text, or
  the form's own hint. A press, or the tooltip's own time running out,
  hides it with a 256-unit fade.
- **Hit test.** With a modal form, it always takes the hit, in its own
  coordinates. Otherwise a point off the screen hits nothing (no form,
  control 1, cursor 0), and the root's test descends to the deepest visible
  form, the last child first. The cursor shape is set only when it changes.
- **Pointer.** An update reads the system pointer and does nothing unless it
  moved (a forced update re-reads it). With a button held, a move past the
  threshold — `(200 − elapsed) / 20` times the drag distance during the
  first 191 units, then the distance itself; the distance is one pixel per
  540 of screen height, at least 2 — sets the drag bit and sends the target
  a 0x4000 press. An idle pointer hit tests and focuses its target; a
  captured pointer sends its moves to the capture, which ends on 0x80.
- **Press.** A second button while a capture holds cancels through the
  release. A press outside the modal form (only possible through a capture,
  since the modal form takes every hit) closes the modal form. The same
  button again within 450 units and one pixel of the press point sends
  0x8000, then the plain 0x1000 when that is answered 0 or 0x10, else the
  double click consumes the button's release. A form answering a press with
  anything but 0x40 holds it, is brought to the front, and with bits 1 and
  4 takes the capture; 0x100 makes its control a drop source.
- **Release.** Only the held buttons' release counts. A press nobody held
  and that did not drag clicks (0x2000); a held press goes to its target's
  release; a capture gets the release, keeping the capture on 0x40 and
  dropping it on 0x80. A drop ends with the payload handed to the form that
  accepted it, or the source told nobody took it. The pointer then hit
  tests and focuses again, falling back to the focus's own form and control.
- **Message box.** `message_box_plan` builds the original's box: the text's
  width plus 64, at least 320, 104 tall, the label at 12, 4, Ok and Cancel
  80 wide and 96 apart at grid row 13, centred (a quarter down with 0x10),
  style 0x808 (0x3000 more with 0x40), the close bit cleared with 0x20, the
  modal form with 0x80; Ok stores 1 and Cancel 0. The layout and the button
  places are `gui_rescale::message_box_layout`.
- **Lengths and grips.** The level switch derives the title (six units less
  two), a title-bar control (four units and one), the step between controls
  (half a unit, shifted, and a control) and a grip (four units less one)
  from level + 3, or 8 at level 4, before it picks the fonts; level 5 then
  raises the unit to 12, so its lengths stay unit 8's. `edge_rect` returns
  the title-bar controls from the right edge less a unit and the grips
  inside the border.
- **Placement and helpers.** `place_form` puts a form at its desired x when
  it fits the screen, else at a fallback x, else flush right less two, and
  lifts it above a fallback bottom when it overflows; `clamp_form` keeps a
  form's x on the screen (flush right when it overflows) and its y at least
  a unit, leaving its size. `set_control_on` switches a toggle: on clears
  its group's on controls (itself included, each reporting unless quiet),
  the lit bit follows the on bit.

### The form base class

`exa_ui::form` carries the form's own rules beside its metrics. A form has
three words: the style word places an unplaced form and anchors it in a
resize (0x800 and 0x8000 centre, 0x20000 alone the right edge, 0x80000
alone the bottom, both bits of a pair stretch), the attribute word selects
the frame parts and the sibling layer (1 title, 0x100 horizontal and 0x200
vertical scrollbar, 0x400 snapping fractions, 0x800 locked vertical scroll
for screen points, 0x1000 upper and 0x2000 lower layer, 0x10000
maximising), and the state word the bookkeeping (low bits 2 maximised, 4
"parent not a task", 0x10 caret lit, 0x1000 layout, 0x2000 restyle, 0x4000
placed). A control record has its own style word (2 not hit, 4 caret, 8
tab stop, 0x100000 no keys) and state word (0x10 disabled, 0x400 focus,
0xf0000 hover).

- **Constructor.** 302 × 402, border 2, content 300 × 800, horizontal step
  0.025 and vertical step 0.25 (the layout recomputes only the horizontal
  one, as `10 / overflow`), the minimum page 76 high and 90 wide, the
  attribute word 0x30022, the state word 0x3000, unplaced, visible, closing
  once, no controls.
- **Placement.** An unset x in the parent's client is `(right − left −
  width) / 2` (truncating) when centred, `right − width − 4` when
  right-anchored, else 4; an unset y `(bottom − top − height) >> 1` (a
  floor), `bottom − height − 4`, else 36. Showing gives an unset y 36
  unless the style centres or bottom-anchors it, lays the form out at unit
  4 and rescales its records to the unit, re-centres both axes (truncating),
  and rebuilds the snap slots of the parent (when not the root), the form
  and each child that is a snap container with children.
- **Sibling order.** Raising a top-level form among the root's children:
  a lower-layer form goes to the first child outside the lower layer (one
  less when that is above it), an upper-layer form to the top, any other
  just below the upper layer. Raising a child in its parent is the same
  except that the lower-layer search starts at the second child, and every
  child below that the raised form owns is raised after it. A new child
  goes above the last lower-layer child, at the end for the upper layer, or
  below the trailing upper-layer children. A form is its task's active one
  when it is the last child outside the upper layer, or, when its attribute
  word is exactly 0x1000, when it is the first child (visibility is not
  looked at).
- **Maximise and restore.** The record saves place, size, border, spacing,
  client edges, words and unit; the form goes to 0, 0 at the screen's size
  with the four anchor bits, a free form (0x40000) losing its border,
  spacing, attributes 0x4f and style 0xf0, the others attributes 0xc000.
  Restoring divides every saved coordinate and size by the saved unit
  (truncating) before multiplying by the current one; the border is
  rounded (ties to even).
- **Resize.** A form with a parent keeps its right or bottom edge when
  anchored there; a stretching child takes the change and is resized, an
  anchored child moves, any other is re-centred (truncating). The records
  follow in 16-bit arithmetic, the centred ones in their owner's client.
- **Fade.** Showing a hidden form starts a 256-tick fade (a visible one
  does not fade in) and raises it; hiding starts one unless already
  hiding. In mode 0 the alpha is `1 − left/256` showing and `left/256`
  hiding; mode 1 computes the eased value and discards it, so the form
  draws at full alpha. A hide that ends closes the form without drawing it.
- **Scrollbars.** The horizontal thumb is `round(track · client /
  content)` (ties to even), at least 9 and at most the track; setting the
  fraction clamps it with ordered compares (a NaN passes), snaps it when the
  attribute word asks, stores 0 when the content fits, and places the
  thumb from the snapped fraction unless the thumb itself is held. The
  vertical track runs from the client's top plus the rail to the height
  less the border and the rail, its thumb `2·unit + 1` when not above
  `2·unit`; its setter does nothing for an unchanged fraction. The wheel
  moves a form with a vertical bar by `round(delta / 120)` vertical steps
  and otherwise its horizontal bar by `round(−delta / 120)` horizontal
  steps, both the same way; held arrows -10/-9 and -13/-14 repeat one step.
  A dragged thumb's fraction is the pointer's offset less the grab over the
  track left for it.
- **Points.** Scroll offsets are the overflow times the fraction rounded
  with `CVTSS2SI`. A root point reaches a form through each ancestor's
  place, inset and scroll; a form-local point reaches the screen through
  the border, the client top less the vertical scroll (not for a locked
  form) and the place.
- **Hit test.** Off the client the skin's frame code answers, and the
  resize codes 0x80000011…0x8000001a set the cursor unless the skin's
  cursor is -2. On it the visible children that are hit are tried last
  first when the content point is strictly inside them; then the records
  that are neither disabled nor style 2 and do not pass the point through
  (0x800000), first on their rectangles (edges included) and then within
  two units (squared distance strictly less). A record answers its 1-based
  index with its 0x7f0000 bits (0x10000 when none) and its cursor; a client
  without a record answers -1, or 0 for a form without attribute bits 0x30.
- **Hover, focus and caret.** The hover clears the old record's hover bits
  and gives the new one the word's; a close-band code (-21, -20, -14, -13,
  -10, -9, -6…-4, and -8 with the skin's first flag) redraws. The focus
  moves the record's 0x400 bit and sets the form's caret bit for any word
  but 0. The caret blinks every fifth frame while the focused record's
  style (or the form's) has 4.
- **Tab and routes.** Tab moves from the focused record to the next
  enabled tab stop, else wraps to the first tab stop before it (the wrap
  ignores the disabled bit); Shift+Tab the other way. A key goes to a
  record as 0x2000 unless it ignores keys, a press as 0x3000 (a drop's
  payload code to the payload's form), a drag to the held record as 0x4000;
  codes off the records go to the parent when the form passes them up.
- **Press, drag and release.** A left press on a grip saves the form's x
  or width and y or height (the codes are bit sets, so corners move two
  edges), on the title (-3) its place unless fixed, on a thumb its offset
  (or the thumb jumps, grabbed at its middle), on an arrow or page it
  starts the repeat. A grip drag keeps the place at 0 or more and the page
  at least 90 wide and 76 high. A release over -6 maximises or restores,
  -5 hides, -4 closes; a snapping form snaps a released thumb.
- **Eager layout and messages.** Showing a form resolves every unset
  place in its whole tree, depth first: a centred x is `(client width −
  width) / 2` truncating, a right x `client right − width − 4`, else 4; a
  centred y is the half height (a floor), a bottom y `client bottom −
  height − 4`, else 36; a place already set is kept. A message sent by a
  control's id reaches the first record with that id anywhere in the tree
  with its message code and two words; with no such record the original's default
  control takes it and ignores it.
- **Snap slots and rescale.** A snap container (attribute 0x20) cuts the
  space between its client's edges and its visible children's widened
  client rectangles into slots: from each free cell the longer free run
  (the row on a tie) becomes a slot one cell thick. A record rescales from
  an old unit by whole old units (sizes made even), anchored records from
  their owner's right or bottom, the y moved into and out of the row's
  centre. The control draw skips records whose state has 0x100 and 0x10.

### The in-game GUI unit

`exa_ui::game_gui` holds what the original's `SG_GameGUI` does without
drawing:

- **Tags.** The inventory is 0xce1c2010, a container popup 0xce1c2013,
  the journal 0xce1c2030 and the loot window 0xce1c2040; the mask
  0xce1c0000 selects that family and 0xce100000 every in-game screen.
- **Open containers.** Eight slots hold the ids (`record id | 0x20000000`)
  of the open containers; a ninth is left out, closing clears the first
  slot with the id.
- **Show rules.** The inventory and journal refuse unless the player's
  controller is ready and its condition is not 0xa00 (dead). An open
  inventory still filling is left alone, an active one closes when the
  call toggles, another is activated. The journal closes when active and
  is activated otherwise. The loot window ("Body", 244 × 440 at x 400 with
  an unset y, so the show centres it at 36) refuses a character whose flag
  word's second byte is 3, activates the window of the same body and opens
  a second window for another body.
- **Inventory.** 244 wide and 644 high (628 compact, 590 in the arsenal),
  titled "Inventory", at (8, 8); the player's own goes to its remembered
  place, another character's to the screen's right less its width and four
  units. The record flag 0x1000000 lights the fourth tab record instead of
  the third; the scene is told the first of the character's items 431,
  500, 451, 452 or 499. Its preview bars are the HUD's layout with the
  record's two health words over 65535; their draw callback is registered
  only when the switch changes. Binding a character to the preview writes
  the model value 0.15 and opening the window 0.05, the scene word 0x1000
  and the draw word 0x42 on every mesh node, copies the 48-byte appearance
  block and registers the model's build callback 100 ticks ahead (the
  opening's 0.05 and 0x1000 are written only when the view has a
  character); the build always keeps the model and answers done, and an
  open inventory showing the view refits its camera to ten times the
  character's height; the destroy releases the node only when one exists.
  The preview bars start 50 units left of the half width and 5 above the
  bottom, 48 units wide and 52 apart, framed 2 pixels out and 4 units − 2
  down; the health bar takes the view's two shares, the focus bar is full.
- **Journal.** 244 × 518 with its text box 228 × 412 at (0, 88), titled
  "Journal", at its remembered place when set. Adding an entry stores the
  text box, appends "New Entry" and selects it; deleting with fewer than
  two entries resets the first, otherwise removes the selection; the list
  renames the selected entry while editing and otherwise stores the text
  and selects the picked row.
- **Controls.** The power slot (0xac3, 40 × 40) answers its hint with 3
  (0x103 for a hovered slot with a count) and hides a visible tooltip;
  a press with 0x2000 on an enabled slot with an icon or a kind opens the
  technique selector (kind 1 answers 0). Its frame is inset by the border,
  its 32-pixel icon is centred (four units in in a 40 × 40 slot), an empty
  slot draws a rounded shadow (radius two units, alpha 0x40) and its kind's
  icon for kind 4, or for a kind past 2 only under the pointer; a hovered
  filled slot draws its icon at full scale with no ink, else the ink is
  state 0x10's, then 0x40's (which also draws the frame again), a kind-2
  slot's (the same bright ink as 0x10), else the plain one. The counter (0xac2, 28 × 16)
  turns a negative value into 0 (−0 and NaN stay), shows `value × 100`
  with one decimal and `%`, and draws `round(width / (n + 1) − 1)`-wide
  blocks whose backs step one pixel and whose fills are three narrower and
  `block + 1` apart. The bar control (0xac4) draws the HUD's two-share
  bar two pixels in.
- **HUD.** The status marker (53 units right of the centre, 8 above the
  bottom) draws while no status entry has 0x2000; the skill-ready alert (60
  units left) while the six skills' levels sum below 15 and no skill has
  more techniques among its first `level + 1` bytes than its level. Both
  pulse with `(3 − 2t)t²` at `t = |1000 − (now − start) mod 2000| / 1000`,
  the glow at 0.6 of it with radius 8 and the icon tint (140, 160, 255);
  a status hide clock is the delay added to the clock, wrapping. The level
  curtain falls to black over three seconds (`round(t · 255)`, the world
  at `1 − t`) and raises the GUI's hold count once it has fallen; the HUD
  reset takes one off that count only from a fallen curtain, then clears
  the state and starts a running heartbeat's fade. The compass
  is a 12-unit diamond two units plus its half from the left and bottom at
  `round((1 − fade) · 192)`. The heartbeat's fade-out lowers its sound to
  `0.3 · (1 − elapsed / 1000)` and stops it; the border keeps its pulse.
  The menu cursor is `round(scale · height / 1080 · 32)`, at most 128, with
  a 0.176/0.08 hotspot. The draw order is the heartbeat, a hook, then (with
  the HUD on and a player) the curtain, compass, marker, bars and gauge,
  then the dialogue and the centred message.
- **Scopes.** Saving the open screens saves a container, a loaded
  inventory or loot panel into its character and the journal's text.
  Closing a family takes the active task's forms front to back, then the
  root's back to front. The deferred close takes the loot windows whose
  character has flag 0x80000 or is no longer dead, in the windows' order.
  The technique selector
  is placed at the record's right and top with its left and bottom as the
  fallbacks; its rows are all disabled when the owner trains and the
  record's level is reached, otherwise the chosen techniques' rows (the
  byte − 1 unchecked, so a byte of 9 marks the forget row) and the
  rows whose requirement is not among the skill's first `level`
  techniques; a positive count opens its forget row.

`exa_game::gui_screens` runs these for the running app: the I and J
hotkeys send a screen request through the hotkey handler (its
permissions apply), the inventory and journal are forms on a `GuiTask`
(so their focus follows the window manager), the inventory is registered
as an open container, the journal keeps its session entries, a right-button
release over the inventory's or a loot window's form closes it (their own
mouse handler), and Escape closes the focused screen (Exarust's). Both
screens now draw
(`exa_game::gui_paint`) with the words their constructors write. The form
attribute word is 0x30022 (the journal adds 0x8000) and the frame style
adds the centred-title bit 0x3000; the item pack adds 0x80 to its style and
0x10 and 0x100000 to its attributes. Attribute bit 1 is the title bar and
no shipped form constructor sets it, so neither screen draws a title bar.
The inventory's 8-unit spacing and the journal's 2 frame the client; the
control records are laid out by the two-pass layout: the inventory's
Encumbrance label (class 5, y 0x62 in design units, at client (4, 386)),
its unlocated class-0xac01 record and its Primary button (108 × 24 at
(4, 416)), and the journal's New (4, 20) and Erase (120, 20), 104 × 24
each. A button caption takes the skin's placement from its style word
(0x3000 centres horizontally, 0x300 centres vertically). The journal's
selected entry draws through the text box. A dead body's double click in
the world opens its loot window (400, 36; 244 × 440): the frame and the
two item panels, the pack 228 × 180 at the client's bottom and the
equipment panel above it ending four units before the pack, each drawing
its entries' icons when the icon cache holds them. The window joins the
open-container registry under the body's tagged id, binds a body's pack and
worn items through `bind_loot`, and leaves the registry when it closes.
The window's flag word is the body's record profession word (race byte 3
refuses), read from its `Possessions` when the window opens; `exa_game::body_inventory` fills the pack
from the body's carried list at its saved places and the equipment panel
from the worn items and the shields in the hands (the original's hand loop
lists an item only when it has a shield section) with the worn/scatter pass,
and the player's inventory pack from its own carried list. `panel_live`
runs the left drag between open panels with the item panel's press, accept,
drop and drag-end rules; a drop on another pack moves the possession
entries, a drop on the same panel repositions, and a drop on the body's
equipment panel or outside every panel is refused. The world's press is
held while the pointer is over an open screen.
Not wired: the character view and its preview bars, the pack's item icons
(the item renderer; a reserved icon slot draws nothing), the journal's
entry list (its geometry is not in the traced units), the 0xac01 record,
the controls' callbacks and the technique selector. Taking a worn item off
a body does not re-dress its mesh, the GUI-to-world drop is not run, and a
level-local item's record is not promoted into the world database, so its
bulk counts 0. The skills key's screen and the power tree are outside this unit,
and the selector opens only from a power slot the HUD does not draw yet.
The Rayform faces are not shipped, so the text keeps the level font's
metrics (glyph quad, advances, line height) and samples its glyph cells
from the base font's atlas, a quad of the level's size over one base cell.
A texture coordinate runs up the image, so a stored-top-down texture is
read from its bottom row.

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

### Text editing

`exa_ui::text_edit` is the single-line editor behind table cells and item
names. Its state is four bytes like the original's: the selection anchor,
the caret (bytes before it), the visible window's end (the last visible
byte, 1-based) and its first byte.

- **Caret from a pointer.** From the first visible byte up to the window's
  end, the caret stops before the first byte whose half advance (an
  unsigned shift) the pointer has not reached, else lands on the window's
  end; it clamps to the text's length.
- **Horizontal scroll.** The window's end, the anchor and the caret clamp to
  the text. A caret before the window's first byte moves the window back to
  the caret less one and recomputes its end from the width. A caret inside
  the window recomputes the end from the first byte, then backs the first
  byte up while the width allows; a caret at or past the end sets the end
  just after the caret and walks back from it until the width is used. A
  byte that leaves less than one pixel ends the window before it.
- **Keys.** A key below Backspace or an unknown key changes nothing and
  does not scroll. Typed bytes replace a selection; a text at the maximum
  takes nothing and does not scroll. Backspace removes the selection or the
  preceding byte, Delete the selection or the byte at the caret; Home and
  End go to 0 and the length; Left and Right move one byte; unless Shift is
  held the anchor joins the caret, and the window scrolls. Only the scroll
  moves the window's end. With Ctrl, C copies the selection and X cuts it —
  with nothing selected both take the whole text — and V pastes the
  system clipboard over the selection (the caret moves past it, wrapping as
  an 8-bit field; the text is cut to the maximum with the caret clamped),
  scrolling even when the clipboard had no text. The navigation handler
  does Home, End, Left, Right and Ctrl+C, nothing at or below 0x22.
- **Draw.** `caret_position` sums the visible advances to the caret and
  backs off half the gap. `draw_edit` has three paths: the whole-run draw
  reads the window's end as a count from the first visible byte; without a
  selection shown the window is one run in the colour already bound; with a
  selection, the bytes before it are in the state's text ink, then a
  rectangle one line tall ending a pixel below the baseline, in the skin's
  selection colour at half alpha, the selected bytes in the selected ink and
  the bytes after it in the text ink.
- **Tags.** `[tcol=RRGGBB]` pushes a colour at the skin's tag alpha and
  `[tcol=RRGGBBAA]` its own alpha (zero included); `[/tcol]` pops;
  `[scrp=i]` takes the main font's slant and another `[scrp]` none;
  `[algn=N]` stores N (a missing digit reads the terminating zero, −48).
  `[rimg=NAME]` draws `NAME.rfi`, its size over six in units, centred in
  the field and hanging from the line, then moves the line down by its
  height less the draw's line step. The texture cache keys its names by
  their first 16 bytes.

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
list under the cell (a modal drop-down of the column's entries); a double
press activates. A press past the last row of a table that grows adds a row and edits it, and committing an emptied
first cell of the last row deletes it. Enter commits, Escape abandons,
Delete on an empty edit deletes the row; every other key goes to the
single-line edit field (typing, Backspace/Delete, arrows, Home/End,
Ctrl+C/X/V), which scrolls its visible window inside the column; a press
inside the open edit puts the caret and anchor there, a drag moves only the
caret (and scrolls by the first column's width, whichever column is
edited). Hovering a row answers 3; hovering no row keeps the last hovered
row. The default row is the first when none is selected, and the table's
centre is half its client's width and height. A header's inset rectangle
runs two pixels in from its sides and top to two pixels short of six units
down, and the drop shadow starts at the header's bottom line, the client's
height less six units tall. Down
passes greyed rows, Up does not. Owner-drawn columns start at the first
visible row. The table never sorts.

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

The view's select slot sets the hovered entry. Dragging over it answers
whether it takes the drop (refused when the drag started on the view
itself or its state turns drops off); the drop and drop-cancelled hooks
answer 0 without a hook. A rename places the caret from the pointer over
the entry's line and scrolls the edit field; check marks draw at the
scroll offset, one row per entry, lit for the hovered or renamed entry;
the selection and the rename field fill the band a pixel inside at a
quarter and half alpha.

## Skin

The base draw skin (`exa_ui::skin`) is the original's `TGUIDrawSkin`: it
owns the palette, the layout metrics and the GUI batches, and draws every
flat-coloured control part. The textured subclasses (the HUD's `guitex.rfi` stone skin, the
`guiskin.rfi` atlas) add their texture binds and shaders; the flat
primitives below are the base class's.

`exa_ui::skin::SkinHooks` names the two of the abstract base's five empty
virtual hooks the skin globals use: the one-time preparation and the hand-off
of alpha and fade (the other three are not traced). The GUI globals keep
the lazily made default skin, the active skin and the alpha/fade pair.
Making a skin active prepares it once (the flag lives on the skin, so
switching away and back does not prepare again) and hands it the pair as
last set (zero before the first set). Setting the alpha stores it as given:
the original calls a clamp helper but drops its result. The alpha byte is
`alpha · 255` rounded to even in 64 bits, low byte. The game skin's atlas
`guiskin.rfi` is resampled to 40 by 20 units (160×80 at unit 4) and made
with the word `0x133`. The textured skin's own file loads with `0x70133`. Its
textured frame draws an edge two pixels in, an edge one pixel in and the
fill; a pressed frame (which wins over hot) reads two slots of the skin's
table twice each, an idle one two slots paired with the next, a hot one a
second pair, and the fill takes the skin's fill ink (its hot fill ink when hot).
Binding a skin texture different from the current one sets `TexCol` to 1
when the texture word has one of the `0xc000` bits, else 0, and writes the
uniform only in renderer mode 2. The per-frame `TexScl` is the screen size
over 512, set unless the renderer already is in mode 4, the mode that setup
leaves behind.

- **Palette.** The scheme record has four colours, each with a scale and a
  tone, an accent, one float and two alpha bytes. A colour expands into
  twelve shades: the base at 4, then the base times the scale, its square
  and its cube at 5 to 7; at 8 to 11 the base pulled towards its luminance
  (green × 0.587 plus red × 0.299, then blue × 0.114) by 1.75, 2, 2.25 and
  2.5 times the tone (at most 1) and scaled by the fourth to seventh
  powers; at 3 down to 0 the base over the scale, divided by the scale
  again, then twice more with a 10/11 step each (multiplied as a double).
  A last pass tones entries 4 to 7 by 0.5, 1, 1.25 and 1.5 times the tone
  (the first uncapped). Scaled channels round to nearest even and saturate;
  toned channels keep their low byte. The scaled entries keep the alpha
  the palette had, as the original stores only three bytes. The accent is
  halved and also scaled by 1.9 and toned by 0.8; both take the second
  alpha byte times the scheme's float as their alpha. Four mixes at 0.4,
  0.6, 0.5 and 0.9 of the way towards the bright end are stored, and the
  last two are blended from the first group's fourth shade by the second
  alpha byte times the float over 255. The first group is copied to the
  hot group. The alpha ramps give the form group and the last two mixes
  the first byte times its scale, and all forty-eight entries of the
  control, hot, label and highlight groups the second. The layout metrics
  are border 2, pad 2, title 24, reach 4·unit + 4, edge 2, gap 2, head 24
  and row 20.
- **Batches.** Geometry is gathered between a begin and an end in window
  pixels, with a current ink and texture coordinate, a program (flat,
  textured with `TexCol`, rounded with the `Scale` uniform, or the
  subclass's own) and a blend word. Selecting the mode already selected
  binds nothing; mode 0x10 asks the subclass, and its answer becomes the
  selected mode. An end submits quads as two triangles per four vertices,
  lines as consecutive pairs, or triangles as they are. All runs share one
  vertex list that a frame clears and refills without reallocating. A
  texture offset in 512ths shifts sampling; the scroll bars set it while
  their thumb or trough is drawn.
- **Submission (`exa_ui::gui_draw`).** A frame is a list of layers, each a
  batch and the texture its textured primitives sample; the layers and the
  primitives inside them are drawn in submission order at window physical
  pixels. Solid mode draws the vertex ink; textured mode samples the
  texture, mixes it by `TexCol` and multiplies it by the ink; rounded mode
  cuts the alpha from the interpolated corner coordinate and the `Scale`
  uniform; mode 0x10 asks the skin's subclass, which answers the flat
  program for the line skin and mode 4 (the stone, sampled in screen space
  from the window's bottom) for the HUD's. The blend word is the
  original's: bits 8–11 pick the source factor and 0–3 the destination from
  a ten-entry table (`ZERO, ONE, SRC_ALPHA, ONE_MINUS_SRC_ALPHA, SRC_COLOR,
  ONE_MINUS_SRC_COLOR, DST_ALPHA, ONE_MINUS_DST_ALPHA, DST_COLOR,
  ONE_MINUS_DST_COLOR`); a word of 0 draws as 0x203, and 0x100 draws with
  blending off. Quads share their diagonal under a top-left fill rule so a
  blended edge is drawn once, and the canvas is kept when a frame matches
  the last. The running game
  rasterises these primitives on the CPU into one window-sized RGBA image
  and shows it over the 3-D view as a UI image; colours are blended in sRGB
  as the original's interface blends, and the image carries straight alpha
  (the HUD's linear-light note above applies). Text and icon quads join the
  frame as their own layers. This is the first submission path; it is used by
  the in-game inventory and journal.
- **Primitives.** Line rectangles inset half a pixel (`outline`), four-stop
  vertical gradients (0.43/0.57 and 0.4/0.6 tones), alpha-scaled gradients
  (the alphas keep their low byte), striped bars that interpolate each
  stripe to the next colour, listing selection bands (an outline whose
  middle is half the width less one in, then glow fans reaching into the
  row from its top and bottom edges, wider when selected), rounded fills
  and textured rounded frames through the rounded shader, one- and two-ink
  rectangles, one- and two-pixel line boxes (each top edge a pixel short of
  its right side), two-line bands, four-ink frames whose sides fade from
  the edge inks, four-stop bands, drop shadows and inverted `inset` rings
  that fade towards the middle, fixed bevel rings, clamped vertical
  gradients and their line form (clamped before the lines move in), bevel
  and gradient line boxes whose left mixes meet half a pixel below the
  right ones, and the two scroll arrows: six line pairs in a look's inks.
  Look −1 draws label shade 2, look 0 label shade 5 and look 1 highlight
  shade 9 additively; the faded copies after the fourth vertex take the
  shade before at two thirds of its alpha.
- **Control parts.** A control face is a two-ink inner rectangle, a
  bordered one-pixel box and an outline, in the pressed, hot (the form
  group's copy) or idle inks; its gradient form swaps the first two for a
  four-stop gradient and a bevelled line box. A panel either draws that
  bevel and outline with an inner shadow or the frame slot. The style
  frames are double frames in the hot group's shades 0 and 7, or 6 and 0.
  A form mark under the pressed mouse draws a bevel just outside its
  rectangle, the outline and a faded face a pixel in; the mark under the
  mouse draws that bevel, a second one a pixel inside and the outline. Its
  icon sits half a unit in from the left and half a unit and one below the
  top, at scale 1 when the mark is under the mouse or pressed with the
  button up. A slider draws a faded track (the hot group's shades 4 and 0
  at 0.6 alpha) and a framed thumb moved by the value, or a striped fill
  and the unit-wide bar at the value; a scroll bar, drawn a pixel lower and
  two shorter, draws a box with a thumb `4·unit + 1` wide (the track
  points back at the same rectangle) or the striped fill. The shell adds
  the frame, an optional drop shadow at `-unit/2` and the sunken face; the
  fill is the control's own colour, forced opaque.
- **Form chrome and background.** A form keeps a state word (bit 1 a
  flat frame, bit 4 no gutters, bit 2 picks the small mark's icon, bit 1
  hides the page) and a style word (bit 1 a title, 2, 8 and 4 the close,
  small and large marks, 0x20 a client fill, 0x40 a rounded shell and an
  outline frame, 0x80 a drop shadow, 0x100 and 0x200 the scrollbars,
  0x800 a locked scroll, 0x2_0000 the scrollbar gutters). The chrome first
  draws the four gutter strips in the form's fourth shade (between the
  margin and the client's left, top, right and bottom, each `bar` wide,
  when there is room; a titled form has none above), then the client fill
  when the form keeps no line rectangles, then its frame: the first style
  field draws a double frame or, with style bit 0x40, an outline a pixel
  in; the second frames the client two pixels outside. A flat form draws
  only its frame: the outline, the double frame or a two-ink outline. The
  default background adds the rounded textured shell, a title gradient
  from y 2 to 24 with an optional two-line band below it and the close,
  small and large marks (their rectangles from the form's control-code
  switch, pressed under the button or hovered), then the vertical and
  horizontal scrollbar frames, their shadows, the arrow frame slots
  (pressed when held), the thumbs (the vertical one as a gradient bar, the
  horizontal one as a frame slot, each with the texture shifted by the
  thumb's place), the arrow icons half a unit less one in, the mark icons
  and last the title's two text blocks: the form's second shade one pixel
  left and above, the label's seventh at the place, at y `(margin + line
  height + 25) / 2`. Their width drops by 17 for the close mark, 20 for
  the large one and 17 for a title icon; the title is centred when the
  style says so.
- **Form page and clipping.** A page draws its own background or the
  skin's, stops if its state hides it, clips to the client rectangle (the
  original scissors to the parent's client), works out the scroll offset
  (the stored scroll less the client's extent, times the view scale; bit
  0x800 locks it at zero) and translates the contents to the inset and
  top less that offset, then draws the line rectangles in the form's
  fourth shade, the control records, the visible children, and last one
  drop shadow (unit and alpha 0x30, or 4·unit and 0x50), locked or not. A
  form whose class asks for it moves the skin's texture with its place.
- **Labels.** A label's place follows its style word: 0x1000 left of the
  control, 0x3000 centred by the measured width, 0x4000 right, 0x5000 past
  the right edge, otherwise the inset and half the unit plus one; the
  vertical arms are half a unit up, centred on the line height, at the
  base, below the control, or the same default. The variants draw a
  focused label in the label group's third shade and an unfocused one in
  its seventh (the slot's idle label in its ninth); the caller's-ink form
  uses the fourth when focused. A coloured label glows: its text in the
  eighth shade at the measured place, then four copies a pixel left,
  right, below and above in the highlight group's first shade, additively.
  The slot's pressed label draws a shadow pair (control shade 2 one pixel
  left, label shade 7 one below); the block form keeps only the bytes that
  fit (`(width − 2·inset) / step`) and right-aligns for style 0x4000. A
  fade scales the seventh shade's alpha by the elapsed third of a second
  (a start of 0 is now) and clears the fade bit once it passes 1.


### Line skin

The game's GUI skin is the original's `TGSLines`, a thin-line subclass
of the draw skin. `exa_ui::skin` models most of it beside the parent:
the opening, the vertical scale, the hit test, the slider and scroll bar,
the frame, the chip, the rail, the shade, the four-unit square, the
capped and plain span bars, the label and the widget dispatcher
(`dispatch_form`). `exa_ui::line_skin` holds the remaining methods.

- **Lengths.** The skin keeps five unit lengths, 2, 1, 2, 2 and 4 units.
  The parent constructor calls the setter through the object's table and
  the line constructor then rewrites the first four, so an opened line
  skin keeps the fifth at four units. A once-only flag registers the
  unit.
- **Hit test.** A point in a window's title band (style bits 0x30 equal
  to 0x10, y within four units, or six on the wide setting) returns -4
  at the close corner and -3 on state bit 1; any other point, that one
  included, goes on to the strip test of state bit 0x200, which returns
  -8 beside the client, -10 below it and -9 above it, within the same
  span. The edge sums are 64-bit.
- **Centred icon.** The line skin places its icon as the parent does
  (half the spare, toward zero; a marked state with bit 0x40 inside 0x60
  one pixel lower, drawn at scale 1 with no ink) but picks its own inks:
  state bit 0x10 the label group's fourth shade as stored; with style bit
  0x8000_0000 clear the eighth shade on state bit 0x40, else the fourth,
  both opaque; with it set the seventh shade with its alpha raised by
  0x40 (wrapping in a byte) on bit 0x40, else the sixth as stored. Style
  bit 0x4000_0000 picks the icon manager's other draw, whose texture runs
  right to left.
- **Scroll refresh.** A bar window without style bit 4 takes the edge
  mask 0xf; with it, bit 1 marks a top above the unit, bit 4 a right edge
  within a unit of the owner's room and bit 8 the same for the bottom
  (the refresh never sets bit 2). The rail starts five units below the
  client top with bits 1 and 4, else four, ends five or four units above
  the client bottom with bits 4 and 8, keeps a side of two units, and
  places the thumb at the start plus the vertical fraction of the span
  less the side, rounded half to even; a NaN slide is 0.
- **Progress strip.** A titled window (unless state bit 0x80_0000 is
  set) and an untitled open bar draw a strip from two to four units down:
  a cap at each end eight units in and the track between them. A titled
  window splits the track at its title's gap, half of the width less the
  title and five units, and puts a cap on the inner side of each piece.
- **Close-edge tints.** With state bit 0x200 a bar draws an upper and a
  lower tab beside the client and a square below the scroll thumb. Close
  mark -9 tints the upper tab, -10 the lower one and -8 the square (which
  binds the marker's sample) in 255, 204, 153, 255.
- **Maximised window.** A window style with state bit 0x40_0000 draws a
  frame of its own: an outer top bar (only with style bit 4, which also
  binds the hot ink on style bit 0x40, and not when the window sits within
  a unit of a focused owner's top), the top, left and right bars, the
  bottom bar and four corner arms reaching two units outside and six in.
  The side bars run four units from the corners, or two with style bit 4.
- **Integer span bar.** The compact skill screen draws a capped span from
  integer arguments with the shared skin and no ink of its own: caps four
  units tall from the top, the middle from two units down.

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

- **Layout.** `layout_records` places the records not yet laid out in two
  passes. First, in order: the first record's x and y below `-0x7ff8`
  become the unit; a later record's negative y moves from the y above by an
  anchor's rows (`-0x8000` 0, `-0x7ffe` … `-0x7ff6` 13 down to 5; up with
  style 0x80000; another value repeats the last anchor's rows); its x copies
  the x above (`-0x8000`), follows the record above on its row by 4 or else
  starts at 0 and moves to the leftmost earlier record above it at or right
  of that x, scanning back and stopping at a record higher than the best
  (`-0x7fff`), or follows the record above's right edge by a gap
  (`-0x7ffe` … `-0x7ff6`: 0x48, 0x38, 0x20, 0x14, 0x10, 0xc, 8, 4, 0). A
  known class fills a missing size (the width snapped to four pixels,
  half to even), style and margin (1 for the icon button and picture) and
  initialises a trackbar. Then each is centred in the client (0x800), right
  aligned (0x20000 in 0x30000) or snapped; its y is counted up from the
  bottom (0x80000 in 0xc0000) or scaled by four, less half its height over
  16 (4 when taller than 32); a frame style adds 1 (0x30) and 2 (0xc0) to
  its margin, and the record is marked. `append_records` keeps the records
  up to the last one in use, drops any after it and appends; the shipped
  registry order is button, check box, image button, label, trackbar, icon
  button, picture.
- **Label draw.** A label draws a panel inset by a pixel with style
  0x10000000 and the skin frame with any of 0xf0; its front is the held
  variant while a button bit of its state is set, else the alternate with
  style 0x20000000, else the pictured or the plain one. A check box's
  label sits five units in, in the disabled ink while disabled.

The extended controls (`exa_ui::controls`) are registered in this order,
with their records' default sizes: text field 80 × 24, combo box 80 × 24,
colour swatch 24 tall, spinner 60 × 20, progress bar 24 tall, separator 3
tall, image, texture view, trackbar 28 tall, caption 20 tall and scroll
bar 16 tall. The text field, combo box, spinner, trackbar and scroll bar
have creation hooks. The unit also installs a unit-change hook (ahead of
the previous one, which it calls) that caches the spinner's button width,
`3·unit | 1`. Each class answers the GUI's messages (draw back, draw
front, pointer query, move, press, release, key, focus, the gamepad
accept, set value, refresh, reset). Rounding is `CVTSS2SI`, half to even
in 64 bits; where an `int` is kept, an indefinite (NaN or overflowed)
result reads as 0. Their rules:

- **Text field.** The usable span is `w − 2·pad − 2·unit`; creation turns a
  zero length limit into 255. The edit state is an anchor, a caret and the
  drawn run (start, end). A press puts the caret at the first byte of the
  run whose half advance (an unsigned shift) passes the pointer's offset
  from the aligned text origin, else at the run's end, clamped to the text;
  a left press in the drag state also drops the anchor there. Enter with an
  owner callback calls it with the submitted bit (and gives the focus up
  with style `0x2000000`); an editable field (style bits `0xd`) passes
  other keys to the text edit unit with its limit, dropping a typed
  non-digit when digits-only (`0x4000000`) and calling the owner after each
  edit with `0x1000000`; a read-only field passes keys for moves only.
  Gaining the focus selects the whole text; the reset message runs from 0
  to the end with the caret there. With a frame bit (`0xf0`) the inner
  frame runs from one pixel in at the left and right and two at the top
  and bottom.
- **Combo box.** The span is `w − (4·unit|1) − 2·pad − unit`; the arrow
  sits `(4·unit|1) − unit/2 + 1` in from the right and `(unit+1) − (unit&1)`
  down, unless the style hides it (`0x8000000`). An editable box shows the
  text cursor left of `left + w − (4·unit|1) − pad`. A left press in the
  edit state drops the anchor, any other opens the drop-down list. Setting
  a selection of 0 (or with no items) clears the text; otherwise the item,
  capped at the last, becomes the text, cut to the limit, and the owner is
  told unless the caller asks for quiet. Creation takes the selected item's
  text and turns a zero limit into 255.
- **Drop-down placement.** A list that does not fit under its owner counts
  the whole rows that fit inside `2·pad` below and above. With fewer than
  six below and fewer than half again as many as above (the half an
  unsigned shift, so a negative count never qualifies) it opens above with
  the rows above; otherwise it is cut to the rows below. A cut shorter than
  the list scrolls.
- **Colour swatch.** A left press with the `0x4000` modifier bit starts a
  colour drag (result `0x105`): the payload code `0xcd0c`, the colour, the
  press's offset into the swatch and its inner size, drawn under the
  cursor at alpha `0xa0`. With `0x2000` the owner is called instead. The
  swatch accepts only the colour code and takes the dropped colour.
- **Spinner.** The span is `w − (3·unit|1) − 2·pad − 2·unit`. Creation
  turns a zero step into 1 (the integer 1 for an integer spinner) and a
  zero maximum into 100.0, written as a float even for an integer spinner,
  and raises a float value under the minimum. A commit clamps the typed
  value below the minimum and above the maximum; a NaN passes both clamps
  and never lands, since the value changes only when it moves by more than
  1e-8 (in double precision). Unless reformatting, a clamped entry marks
  the text dirty. A press left of the button column puts the caret (or
  selects all with the `0x8000` modifier); on the buttons it steps up
  above half the height (an unsigned shift) and down below. The buttons
  are half the height plus an overlap of 1 (2 from a unit of 5) high, the
  arrows `(unit − 1)·2 | 1` wide. Typed bytes pass as digits, a dot (a dot
  over the dot at the anchor steps past it instead) or a minus while the
  anchor or the caret is at 0; Up and Down step, Enter commits and selects
  all, Ctrl+V pastes and Ctrl+C copies. The drag step is `96·unit /
  ((max − min) / step)` pixels, at most `8·unit`, then at least `unit / 2`.
  Once the text drag has moved past the unit it captures the mouse over
  the value's whole reach (the two clamp limits; the other two region
  fields are the sentinel `0x80000001`) and remembers the cursor and the
  value there; further moves step the value, the float by
  `round(Δy / pixels) × step` added to the remembered value (in that
  order), the integer by the same product added to the remembered bit
  pattern. A point in the button column marks the up or down half
  `0x1000000`/`0x2000000` when the state word enables it with
  `0x4000000`/`0x8000000` (above the top counts as up; outside the column
  clears both); a capture's bounds come from the current value. The set message's word is an integer
  converted to a float unless its flag marks it a float (an integer
  spinner stores the word's bits as they are), clamps with the ordered
  compares and lands only a move over 1e-8.
- **Progress bar.** Setting a negative value stores zero (−0 and NaN stay)
  and tells the owner every time. The fill is `w × value` rounded half to
  even; the empty part draws at 0.4 alpha, the bar and its highlight two
  and one pixels inside the pad. The caption is `value × 100` with one
  decimal and `%` (the formatter is outside the evidence), drawn only on a
  bar taller than five units.
- **Separator.** With no text the line spans the control, raised by half
  its height or, with a line count, lowered by half of `unit × count`. With
  text (a gap of the font's character width per byte) a right-aligned
  (`0x2000`) separator keeps only the part right of the text and a unit,
  any other leaves the text between two `(w − gap) / 2 − 1` lines. It, a
  caption without a callback and the image let the pointer through
  (`0x800000`).
- **Image and texture view.** A drag over the image picks
  `u = (x − pad) / (w − pad)` and `v = 1 − (y − pad) / (h − pad)` in double
  precision from the message's coordinates (the control's origin is not
  taken off), each clamped below 0 and above 1; a NaN passes. The texture
  view centres its texture's cell, each half truncated toward zero, or
  fills the inset with its colour when it has none.
- **Trackbar and scroll bar.** The trackbar's creation hook defaults the
  knob width to 20 and the pad to 2, and a unit change rescales the knob
  in double precision. Its value is `(x − left − pad − knob/2) / travel` in
  double precision, clamped below 0 and above 1 with a NaN passing, then
  quantised by its step count. The scroll bar's creation hook sets the pad
  to 2; its thumb is `4·unit + 1` wide, its value maps the same way, and
  its captions are its text at the left, its suffix at the right and
  `(max − min) × value + min` with no decimals in the middle. An enabled
  bar re-sends a left press as a drag.
- **Caption.** The skin draws behind it only with a frame bit; the text
  wraps with either `0x3000` bit; an enabled caption calls its owner on the
  `0x2001` press.
- **Label alignment.** The horizontal switch has cases 1, 3, 4 and 5 only:
  left of the box, centred in it, ending before the right edge, and past
  the right edge. The vertical switch likewise has 1, 3, 4 and 5, centred
  by the line height (halved by a shift) or the font base. A 2 is the
  default.
- **Drawing.** `controls` lays each control's draw out as the skin calls
  it. A spinner's field runs to its button column, its inner rectangle one
  pixel in; the buttons are half the height plus the 1- or 2-pixel overlap
  (2 from unit 5) and the arrows `(unit − 1)·2 | 1` wide, `(unit + 1) / 2`
  into each. The up button's press bit is `0x1000000` and the down's
  `0x2000000`; while the control is hot both arrows take `0xffffffff`,
  else the up arrow takes 1 on style `0x20000` and the down 1 on `0x40000`.
  A progress bar draws its empty part (`fill` to the right edge, one pixel
  in, alpha 0.4), its four-stop bar two pads inside the fill, and its
  one-ink highlight one more pixel in. A separator's line rectangles come
  from its gap; its ink slots are `0x9c, 0xa0, 0xac, 0xb4`, or `0xb8,
  0xb0, 0xa0, 0x9c` with style bit 0x40. A combo box's arrow sits as above;
  state bit 0x10 picks the `0xd4` slot, the edit state `0x10000` draws it
  through the textured path and its text through the text edit unit, else
  the `0xe0` arrow and the `0xe4` text. A trackbar's track is inset two
  pixels and drawn at alpha 0.6, its knob at `x + pad + rint(travel ×
  value)`; a scroll bar's track is the same inset and its thumb `4·unit +
  1` wide. An image's shade fades grey 128 from alpha 0 at the top to 255
  at the bottom; a texture view fills its inset in its stored colour or
  centres its cell; a colour drag draws its payload at alpha `0xa0`. A
  text field draws its inner frame (style `0xf0`) and its aligned run; a
  scroll bar draws its text, its `0x4000` suffix and its `0x3000` value.

## Skill screen

The skill tree (`exa_ui::skill`) has six groups of eight nodes. A node code
keeps the 1-based group in bits 8–15 and the 1-based level in bits 0–2.
The record stores the points, the chosen techniques and the prerequisite
table; the screen also keeps each group's committed points (before this
visit's experience). The screen reads the record's skills as its byte
layout (two point bytes, six technique bytes a skill), so its loops that
pass a sixth level read the next skill's bytes, as the original's do.

- **Ring.** The ring page shows the groups in the slot order Close Combat,
  Armour, Insight, Concentration, Shields, Ranged Combat; a positive page
  shows four groups at a 72-unit radius stretched 1.25 across (64 and 0.9
  with the inset-layout flag). A slot sits at angle
  `(1 − turn) + (slot − 3.5)·2π / count`, its offset the radius times the
  unit times the engine's sine across (stretched) and its cosine down,
  each rounded; the radius is 64 units, or 58 with the global inset-layout
  flag (which also moves a page's ring a unit left). The engine's
  sine-cosine helper returns the sine first. A group's pentagon points are
  18.3 units out at fifths of a turn from straight up, the sine across and
  the cosine up, rounded in double precision.
- **Press state.** A level the points fund is learnt (a press asks to
  forget it). The next level is open (a press opens the learning row)
  while a level is ready and no technique is chosen there; with one chosen
  ahead of the points it is learnt once the skill is more than ten points
  past its funded levels, else still open. Later levels are locked.
- **Learning row.** Its requirement mask marks the five written techniques
  chosen, the slots the tree has no node for absent (Ranged Combat has
  none; Close Combat's fifth to seventh, Armour's sixth to eighth and
  Shields' seventh and eighth), then every open slot whose two
  prerequisites are not both among the funded techniques missing (Dual
  Wield wants Remise and Riposte). A row slot is hit in the eight units
  above the node, ten units apart; a second press on a clear slot first
  unlearns the record's surplus choice, then writes that slot's technique
  into the pending level. The row's box is 82 × 28 units (its width eased
  by the popup), four units under the node, with eight icons, a set
  requirement byte drawing at half alpha, and the hovered slot's name over
  its description or its status: "Already chosen.", "Requirements not
  met." or "Unavailable.".
- **Forgetting.** The forget dialog ("Forget Technique", "Forget all
  knowledge of this technique?") is a glow box 82 × 20 units two units
  under the node, Yes 12 units left and No 6 units right of the centre one
  13-unit line under the title; its arrows are hit six to ten units under
  the node, beyond four units either side, before the learning row. Yes
  unlearns the level with the screen's rule: at or below the committed
  levels both point counts lose 5000; one past them the committed points
  round down and the record's lose 5000 while it holds more levels, else
  take the committed points; further out the record's lose 5000 when they
  fund the level, else round down. The level's technique leaves, the later
  written slots move down, and from the last funded level down every
  technique listing the removed one as a prerequisite is unlearned in turn
  (Armour I takes II and III). The arithmetic is 16-bit.
- **Popups.** The learning row and the forget dialog open over 300 ms (the
  alpha `t(2 − t)`, the width eased again) and close over 200 ms (`(remaining
  × 0.005)²`), after which the pending choice clears.
- **Ready level and experience.** A locked screen is never ready. With
  global experience, the screen first spends its worth: the last group
  with a partial committed level is topped up (the missing points times a
  weight of four plus the committed levels, quartered), then each level
  between a group's committed and record level costs `weight × 5000 / 4`,
  the weight rising after each, and what is left (even below zero, the
  comparisons reading it unsigned) goes to the record's surplus choice. While
  experience is left over a level is ready; otherwise the scan stops at the
  first skill whose next technique byte is set, which must be no more than
  ten points past its levels, and the levels counted to there must stay
  below fifteen.
- **Group icons and orbits.** A group shows one icon per funded level, plus
  the next while a technique is chosen ahead of the points (at half alpha)
  or a level is ready with fewer than five funded, about a point four units
  left of and above its ring position; the first empty level draws the
  "choose" icon and ends the row. Its orbit bars span 20.3 to 26.3 units
  out, 14 either side, the last filling by the points past the funded
  levels when below 0.99 (in double precision). Behind each node the draw
  puts five spokes 12.3 to 26.3 units out, 18 either side. With no record
  the screen previews a skill of 13 200 points with techniques 1, 3 and 5.
- **Hover tooltips.** A ring node's tip is 82 × 14 units centred 13 units
  under it and clamped into the window; it names the technique chosen at
  that level, or the group with none. A group's tip is its name's width
  plus two units, the four units under the node, unclamped. Tips slide in
  and out over 200 ms. The icon atlas cell of icon `n` is `n − 1` counted
  along rows of `width / cell` cells, its texture size divided in double
  precision.
- **Fades.** The screen reveals over 500 ms from its creation (the alpha
  `t(2 − t)` capped at 1, the ring's spread that eased again) and hides
  over 256 ms. The draw keeps the content alpha and the opening scale
  apart: the ring's plates, spokes, icons and orbit bars multiply their
  offsets by the alpha, while the dim, the shader alphas, the icon ink and
  the tooltips take the scale; the hide scales the scale, not the alpha.
- **Draw order** (`exa_ui::skill_draw`). A page-2 screen first draws its
  background texture over 12 × 8 units at the centre. A ring page dims the
  view black at `77 × scale` and then every node's soft plate (30 units
  either side, black at `0.6 × scale`) and its five spokes (12.3 to 26.3
  units out, 18 either side, colour 1, 0.98, 0.9 at `0.4 × scale`); the
  sine and cosine are the engine's and the reveal alpha scales both. The
  node's own group icon is 24 units a side one unit above the node's top
  half (ink 255, 248, 240 at `120 × scale² × 0.75`), then the small level
  icons (8 units; ink 255, 248, 240 at `120 × scale`, halved on a chosen
  technique ahead of the points, 255, 204, 153 on the hovered one), then
  the orbit bars (colour 1, 0.79, 0.286 at `0.5 × scale`). Last come the
  ring tooltip, the learning row (eight 10-unit icons in its eased box) or
  the forget dialog (its title, the question and YES/NO at
  `(alpha − 0.5) × 2`). The two shaders are the rounded ring source and
  the bar source, both with the fixed `MPrj` projection matrix; the bar source also takes
  `Tex` at unit 0 and `Prog`, and the screen's setup loads the
  `SkillShow_01` reveal sound.

## Item windows

The item GUI's rules, read by REA from the original's item panel, icon
cache, inventory, arsenal and item-card code. The running inventory screen
paints its pack panel and, when the item manager's icon atlas has icons,
the pack's icons through the GUI draw path (below); no item renderer fills
that atlas yet, and the other windows wait for their forms.

- **Panel entries** (`exa_game::item_container`): an entry is drawn
  unless it is dragged, held (`0x100`, the item in a hand) or has no icon,
  tinted green by the panel's green mask, yellow at half opacity by the
  yellow mask alone, green at half opacity by both, else black. A load
  keeps each icon's centre its half extent inside the panel (right, left,
  bottom, top); replacing an entry's item moves its bulk and keeps it
  inside (right, left, top, bottom). An item given back to an open pack
  that still lists it as held is shown again; the dragged entry taken
  into a hand becomes held. A looted body's worn panel scatters its
  items and shield (leaving out any the icon cache cannot draw, which take
  no random draws) over the middle half of the panel and relaxes them in
  four rounds (overlap ÷ 6, 5, 4, 3, twice each), measuring both axes
  with the icons' horizontal half extents as the original's loop does. A
  map opens as a parchment of its texture's size in sixths of a unit.
- **Layouts** (`exa_game::item_container`): the grid takes `round(√(n ·
  W/H))` columns and `ceil(n / columns)` rows (single precision), gives
  its first slot to the last entry, jitters each centre by up to an eighth
  of a cell from its own seeded generator and relaxes five times (overlap
  ÷ 4 … 8), clamping after each pass. Scattering everything and placing a
  new entry draw from the engine's shared generator; the scatter relaxes
  six times (÷ 1, 1, 2, 2, 3, 3) and the placement six passes of two
  rounds, pushing only when an axis overlaps by more than the pass number
  (a signed compare) and only when both axes do. The scatter and worn
  passes also store a seed for the run-time library's own generator.
- **Panel state** (`exa_game::item_container::Container`): a panel keeps
  its entries, used bulk, capacity, item limit, container id, linked
  panel, owner, masks, vertical scale and client area. A list load stores
  positions in sixteenths of a unit (y also divided by the vertical
  scale) and clamps every resolved icon; a record load takes the
  container's id and positions in eightieths and clamps nothing; both
  round half to even and the saves keep the low 16 bits. A record is saved
  only for a loaded panel showing a database copy. An add at the centre
  refuses an item with no icon (the original's user-item error). A drop
  from the linked panel shows its held entry again, a drop from the panel
  itself moves the entry still marked dragged with the payload's icon, and
  either is raised; any other drop appends the item, adding its bulk unless
  it came from the linked panel. The dropped entry is kept inside by its
  icon's half extents (right, left, top, bottom). Item 499 appended fires
  game function 2 with the container id, or 3 for an owned pack. A drag
  released elsewhere snaps back with the payload's flags when drag-out is
  refused, leaves without charging bulk for the linked panel, and
  otherwise leaves and loses its bulk. A UI-scale change rescales
  positions by `round(unit · place / old)`. The hit test tries the icons'
  own pixels from the top, then (a switch on by default) the icon cache's
  second test; nothing happens to a mouse event without its press bit.
- **Container records** (`ContainerRecord`, `ListChild`): the panel reads
  the container section's size, capacity and item limit words and its
  12-byte children `{id, flags, x, y}`; the character list is an array of
  the same 12-byte entries. Both parsers are total and both writers
  round-trip the fields the panel uses.
- **Item resolution** (`exa_items::item_gui`): an id is looked up as an
  object record first and then as an icon; a missing record or icon
  produces "Unable to resolve item: XXXXXXXX" (none for id 0). A popup
  container's inner panel is its record's size times the unit plus twice
  its border (`exa_game::item_container::popup_inner`).
- **Cards** (`exa_items::card`, `exa_game::item_container`): a card
  request with an item under the pointer and no request pending asks for
  the card on a 200 ms timer (two of the GUI timer's 100 ms steps) when
  the item's use type is 0 or 1, runs the game's use first for type 2,
  and builds the card at once otherwise. The weapon page shows weight
  `w·0.01`, balance `b·0.05` and the handling `(a + f)·5` with
  `a = (g − 127)·0.01` and `f = ((1 − b·0.01)·0.6 + w·0.004 + 0.1)/1.04`
  for a balance byte up to 100, `(w·0.004 + 0.1)/1.04` above, in the
  original's mixed precision; a weapon flagged `0x6000000` shows weight 1,
  handling 5 and balance 0. Strike rows show above 5 and scale by 0.05;
  apparel rows above 0.25 and its weight always. The striking side's shown
  rows (slash, crush, pierce) fill the card's rows from the first. A
  description's pronoun marker names `HeShe`, `HimHer`, `HisHer` or
  `HisHers`, giving he/she, him/her, his/her or his/hers by the owner's
  sex bit; the description box sits `0xb4` pixels below the page's top,
  4 pixels in, the page's width less twice the border and 8, and is first
  `0xa0` tall. Refreshing an open card rewrites the strike rows (each
  non-zero slash, crush and pierce byte into the next bar from 0xd03, the
  builder's 5-or-less rule not applied),
  thrust (0x106) and the value (0xa01); the apparel refresh also rewrites
  weight (0x101), impact (0x102), the strike rows (0xd03–0xd05) and
  encumbrance (0x107, 8 at a byte of 0xa1 or above).
- **Card content and window** (`exa_items::card`,
  `exa_game::item_container::CardWindow`): the weapon page's rows use
  controls 0x101–0x103 for weight, handling and balance, the shown strike
  rows continue 0xd03 upwards and thrust is 0x106; the material, socket,
  rank, points and value rows add 0x14, 0x18, 0x14, 0x14 and 0x18 to the
  page's height (the material only when more than two striking rows,
  thrust included, show). A weapon's points row shows while the skill's
  low five bits are set and reads them less one (not below 0) when the
  item flags' `0x30` bits are `0x10`. The value row has its own gate. The
  apparel page's impact row is 0x102 and its encumbrance 0x107; the
  shield page shows 1, 0.05·byte and 8 (fixed, with no rank, points or
  value rows) or the bytes' 0.01/0.05/0.05. While the rank gate is off,
  the weapon and apparel pages tint the title between the palette's low,
  middle and high words by the condition byte, over the low-to-middle
  pair below 0x80 and the middle-to-high pair above, with factors 1/127
  and 1/128 in opposite orders for weapons and apparel (a model whose
  flags carry bit 2 maps the whole byte), each a double product; the
  blend rounds each channel half to even. A request is delayed
  200 ms; a close cancels a pending request, otherwise an open card fades
  over 0x100 ms, and a request while one is pending drops it. A double
  click's use class 0 opens a container (and does nothing for anything
  else), class 1 closes the card, and a class above 1 closes it and runs
  the game's use hook; `panel_event` checks the drag bit first, then the
  double click, then the right click's card, which reads the use
  section's kind (delayed by 200 ms for kinds 0–2, running the use for 2,
  at once for a higher kind or no section). An already-open popup is refocused; a new one is sized from its
  record (`2·border + cells·unit`) with its inner panel six units plus the
  icon down. The five special controls register after the shipped ones:
  rating dots (0, 24), slider (72, 24), coin value (128, 24), crystal
  sockets (0, 28) and item icon (160, 160).
- **Inventory** (`exa_items::inventory_window`): 244 wide and 644 high (628
  compact, 590 inside the arsenal), the doll above a 228 × 180 pack at the
  bottom of the client area, of capacity 400 and items up to 50; only what
  the character wears counts against the pack when it is bound. A
  right-button release closes it. Not wired into the running game.
- **Arsenal** (`exa_items::arsenal`): up to five 236-pixel columns laid 60
  units apart (240 pixels at unit 4), as many as fit after a 12-unit margin
  (and 61 for the inventory while shown), scrolled by two arrows. The form
  has eight controls on one row: the arrows, five template buttons and the
  inventory toggle. Escape or a right-button release closes it; the
  inventory key shows or hides the inventory. A borrowed item stays in its
  column marked `0x200`, which the inventory refuses, until it is returned.
  Closing saves the five columns into the chosen set, if any. Not wired
  into the running game.
- **Icons** (`exa_items::icon_cache`, `exa_items::item_gui`): one icon per
  item and variant (lit models by their condition byte), counted by
  reference; 40 units square; framed at 70 (30 for apparel, 20 close),
  the weapon light preset for weapons, and given a one-pixel alpha halo
  (the highlighted one also lifts green). Cards show rating dots (whole
  up to the value less a quarter, then a half dot), a slider, coin groups
  and crystal sockets; a popup opens centred on the cursor, its bottom 8
  units below it. The four socket sheets (`dotrating.rfi`,
  `coinicons.rfi`, `wpnflpgrpico.rfi`, `crystalicons.rfi`) keep their
  three, three, three and six cells; the rating dots are stored down the
  sheet at (0, 128), (0, 64) and (0, 0), the coins across at (0, 0),
  (48, 0) and (96, 0), the grip sides at (0, 96), (96, 96) and (192, 96),
  and the crystals at (0, 128), (64, 128), (0, 64), (0, 0), (64, 0) and
  (64, 64). The coins draw into a `4·unit` manager in the colour format,
  the grip sides into an `8·unit` one in the single-channel format and the
  crystals into one of the interface's large-icon cell (22 before a GUI
  level is chosen; Exarust keeps 22); all blend 0x203.
- **Panel drawing** (`exa_game::item_panel_draw`): every visible entry is
  drawn at its stored cell (inside the pack panel's client area, clipped
  to it) from the manager's atlas through the item
  panel's own program, tinted by its highlight masks (green, yellow at
  half alpha, both green at half alpha, else black). The program
  (`exa_ui::skin::SkinMode::HltCol`) sums a ring of eight neighbouring
  alphas, scales it by the vertex's green and 0.75, mixes the highlight
  (1, 0.75, 0.45) to white by the vertex's red, adds the glow over the
  pixel by the square of its transparency and applies the vertex alpha. A socket row hovers by fifths of a
  unit, takes only crystals 431, 451, 452 and 499, shows a crystal's card
  on a right click and starts its drag on a drag, and hands the previous
  crystal back when one replaces it; a finished drag clears the socket's
  kind but leaves its charge byte. Committing the row writes the sockets
  into the item's weapon section (refreshing its electric effect), else
  its apparel section (refreshing the player's abilities), and an AR
  crystal fires the game's fourth function.

## Retail screens and popups

The remaining forms are modelled from the original's own routines, as
plain draw and input records.

- **Tooltips** (`exa_ui::tooltips`): a shown tooltip fades in over 256
  clock units, stays 2500 and fades out over 256; showing one while it
  fades out reverses the fade. The box is its text plus four units wide
  and two units taller than a line, centred on the pointer, right-aligned
  two pixels in near the right edge, and above the pointer when 32 pixels
  plus the box would leave the screen. Its background alpha is 153; the
  text sits two units in and one up from the bottom. The skill tooltip is
  50 units wide (plus a pixel at an odd unit), 18 units tall (24 for the tall form),
  nine units below its anchor, with its title at 2, 5 units and its body
  box at 2, 6.
- **Modal screens** (`exa_ui::modal_screens`): the message box confirms
  on Enter or Space and cancels on Escape with result −1; a click with
  `0x100` confirms, `0x600` cancels. The shader warm-up screen fades in
  over 200 units, steps one cache entry a frame and fades out when the
  cache is warm; the key-capture screen fades the same way, warns
  "ALREADY IN USE" for 1000 units and captures keys, mouse buttons 4, 8
  and 16 (`0x8004`, `0x8005`, `0x8006`) and pad scans.
- **Transition** (`exa_ui::transition`): the fade to black takes 600
  units, or 800/300/2000 by the mode flags, with the music faded
  alongside; then the switch callback, two hold frames, a 300/500/1000
  hold and a 600-unit fade back. The "TO BE CONTINUED" hold raises its
  text overlay to 0.6.
- **Story background picker** (`exa_ui::class_select`): the four
  portraits of `startcharsxa.rfi` at a scale of the screen height
  quantised to 1/135 and scaled by 1/8, 256 by 512 cards 288 apart,
  centred and 140 down from a 1024-high frame, only the unlocked ones
  (the first always), the chosen card cream. The chosen background's name
  sits centred below, with its description wrapped in a 145 by 60 unit box.
  The hit test uses the same layout and the cards' own rectangle. Keys:
  confirm with a card hovered, two keys go back, and left and right step to
  the previous or next offered card.
- **Reply choices** (`exa_ui::dialogue_choices`): a 400 by 120 panel
  eight units above its parent's bottom. The hit test answers the entry
  number from 1; an entry's band is its wrapped line count times four units
  plus one, from one unit down. The height eases from its base to the
  target over 500 units (a pure `3t² − 2t³` share), then the target is
  taken. A left click carrying `0x2000` selects the hovered entry; the
  dialogue unit marks it and holds it for a second, during which both text
  colours' alpha fades with the same ease and, at the end, the entry list is
  cleared. Rows read "N." and the wrapped text, starting a line and a unit
  down, one line height per wrapped line plus a unit apart; the colours come
  from the interface palette.
- **Pickers** (`exa_ui::picker_fields`): the marker is the point times the
  field's client size less one, in single precision and rounded to even.
  The toggle key flips the marker bit, the hide key clears it, and a move
  key adds hundredths of a stick to the point (worked in double), holds it
  to 0 to 1 (an unordered value stays unordered) and runs the callback; the
  handler always answers handled. The pointer pick is the shared base pick
  in `pickers`. The physique field draws a cross of two lines in a grey of
  alpha 0x28, and maps a point (an unordered one, or one right of the middle)
  to the second and third weights, otherwise to the first, third and fourth,
  written into the first float of the generator's first four body-shape
  entries. A press on a free field picks and answers 7; a release clears the
  generator's dirty flag or runs the callback.
- **Popups** (`exa_ui::popups`): the item card is 268 by 260, its fade a
  256-unit squared curve (in: the square of the elapsed share; out: the
  square of the remaining share); its icon is centred, or on the stats
  card thirteen units in and one down, pushed by any icon height over
  eleven units. A left press keeps the card open, a right release or a
  hover answer of 0 fades it out. The container popup wraps an item panel
  (a 544-byte object, margin eight units) and destroys it on close; the
  parchment is a sixth of a unit per texture pixel (so a 64 by 32 map is
  43 by 21 at four pixels to the unit) and is hit-tested through its
  texture's alpha (0xF0) at six texture pixels to the unit.
- **Dialogs and colour** (`exa_ui::dialogs`): the colour window is 402 by
  282 with sixteen layout elements, centred in x and on the upper layer;
  its constructor installs five gradients from black (to red, green, blue,
  red and white) and a six-stop hue table for its sliders. The four
  confirmations are 392 by 112 and close with `0x40` when nothing is
  selected. The arena status panel's emblem is 48 pixels, centred on the
  client and 16 down.
- **Help and task stack** (`exa_ui::help_screen`, `exa_ui::task_stack`):
  a help window is 23 units plus its text tall; its "don't show" answer
  goes into the tip popups' settings mask, and only from a control with
  state bits `0x60`. A release carrying `0x2000` closes it. The two-pane
  variant pages the second box between one and its last page and scrolls
  its text one step with the other two page keys. A GUI task's menu
  items become `0x68`-byte records (the first loses its `0x8009` id);
  focusing a task moves it to the top, and removing the active layer
  clears the published task.
- **Tip banner** (`exa_ui::game_tips`): a translucent box eight units
  down, four units tall and the tip's width plus two units, 160 alpha,
  centred, the tip one unit in; no input. Choosing the tip is the
  prelude's (`exa_game::prelude::Tips`).
- **Journal** (`exa_ui::game_gui`): closing remembers the window position
  and stores the text box into the selected entry.
- **Colour swatch opener** (`exa_ui::colour_swatch`): the popup of
  `exa_ui::col_swatch` is placed to put the current colour's cell (column
  `colour >> 3`, row `colour % 8`, three-unit cells) under the pointer, or
  under the button's anchor (x plus half its width, y plus its height) when
  the anchor bit is set; a colour button's popup uses the `0x6000` action
  set.
- **Doll equipment** (`exa_items::doll`): the record's two weapon sets.
  A re-dress compares the stowed set with the out set hand by hand; only a
  changed hand is detached, then attached by its item's sections: a weapon
  as a weapon part (the off hand's weapon that also has a light section as
  an off-hand light), a shield as a shield, an item with only a light
  section as an off-hand light, and anything else not at all. The item's
  `0x1000000` flag is written to the body's flip byte and `0x2000000`
  lights its held part. The stance restarts with blend 20 after every
  re-dress unless the body is in its weapon-swap state (`0x1F`), and the
  sets then trade only when a hand changed. A weapon's take-out clears
  the record entry that names it (either hand's) and, when it was scheduled
  for the main hand and the weapon was the body's first attached weapon,
  promotes the off hand's light or weapon into it — never a shield — the
  promoted item taking the off hand's flags. An off-hand light's or
  shield's take-out reads and clears the record's off-hand entry without
  comparing its item. A take-in (once the original's item node has
  loaded, which Exarust does not need to wait for) follows the take-in
  order: the off-hand light, the main hand's light moving to the off hand, the second weapon
  only when asked for, and a two-hander freeing both hands. A shield frees the off hand and clears the body's weapon
  part when it holds none.
- **Item renderer** (`exa_items::item_renderer`): a 160-pixel icon
  rendered at 320 into colour and depth buffers, with the full and final
  RGBA images and the private scene and camera.
- **Body loot** (`exa_items::char_loot`): 244 by 440, a 228-wide pack 180
  tall at the bottom of the client area and the equipment panel above it,
  ending four units before the pack. A held weapon dragged off is
  dropped; any other item is taken off the record and the character view
  asked to remove it, which marks the character, plays a sound and may
  rebalance it, and apparel is rebuilt either way. Power Crystal AL (500)
  dropped on the body clears the mark and runs its "Activate" dialogue
  before the item is added. Not wired into the running game.

## Character view

The character screen's 3D panel (`exa_ui::character_view`; the item
window rules above are its drop targets). The figure stands at
(0, 100, 0); the camera sits 410 cm from it towards (−200, 60, 360), so
it looks along (200, −60, −360), with a 30° field of view. The first
super-sampling level is 1 when the setting is at most 0, two more than
the setting for 1–3, and 4 above. The scene is `_Inventory`. Its
background is a vertical wash of the skin scheme's colour: each byte
over 240, decoded from sRGB when the renderer's sRGB targets are on (the
default) or × 1.4 when they are off; red is 0.38 of the green, green 0.4
and blue 0.45 of the blue, as bytes of 255 at the top and 280 at the
bottom, rounded to even. The view's task eases the figure's race share
up to its race's value by 0.003 per frame millisecond, never past it.

The scene's environment is given two colours, grey 0.5 and 0.75, its
mode word 2 and a light level of 1; its view distance is 100 units. Each
frame the figure's node takes the direction (0.5, 0.7, 0.5), already unit
length within the original's rounding, turned into the camera's space and
normalised, so the doll keeps facing the camera as it orbits; the body is
also given the camera's point. A per-frame counter steps down by one and
never below zero.

A drag over the figure is accepted only when its item section allows it:
apparel needs a race that wears it (bit 4) and a view that allows
changes; a withheld weapon or shield needs the matching body permission
(`0x10000`, or `0x20000` for a weapon's second bit); a race whose core
word takes the AL crystal accepts item 500 while nothing is shown. A view
that allows changes answers for any item; a frozen one only while the
pointer is over one of the doll's held items. A drag off the figure refunds
the item's mind cost (charged again by a drop back on the view or a
cancelled release): twice a weapon's desirability, else an apparel's
protection value, the weapon section read first; the shield and the core
crystal have none.

A drop from the view's own panel only charges back the refunded cost.
Otherwise apparel is worn, with both the doll and the shown character's
mesh and textures rebuilt, the character data told of the equip change,
and the shown character's pack (when it is not where the drag began)
charged the item's bulk, locked or not; a weapon is taken into the doll's
hands and mirrored on the shown character; a shield likewise; and a body
that takes the item as its core crystal shows it under the pointer and
clears the shown character's mark. For a weapon or a shield, when the
view is not locked and the shown character's pack is not where the drag
began, the pack also takes the item at its centre, marked held. With the
view's mind-cost flag on, the drop charges the cost first and a refusal
hides the view: the apparel's wear room (its value less the worn pieces
it overlaps), or — only with a second character shown — twice the
weapon's desirability less what it displaces.

A hover shows the item under the pointer and its icon, and clears the
shown item when the icon cannot be resolved. A hit on the shown icon
answers the shown item. Otherwise the pick walks the figure's nodes and
takes the last one under the pointer whose flags carry `0x89`, that is not
flagged `0x4000000`, and whose item is not flagged `0x12000000` while
settled; the record hand entry that the node's slot names gives the item's
flags, else the first carried entry with that item does. (The slot is the
1-based place among the doll's attached items, so its flags are one of the
record's four hand flag words.) A frozen view starts drags
only from a held item.

A release takes a hand's item out: over one of the doll's item slots the
take-out is scheduled on the doll (unless a view byte of unknown name
keeps the doll's hands) and, when a second character is shown, on it too,
and the shown character's pack drops its held icon; the doll is not
rebuilt. A take-out of the main hand's first weapon promotes the off
hand's light or weapon into it (a shield stays), the promoted item taking
the off hand's flags. Otherwise, when the dragged icon is the shown item,
the shown item is taken off the worn list, the shown character's mark is
set again and the shown item cleared, with no rebuild; and for anything
else the dragged item is taken off, the shown pack is charged its bulk,
and both characters are rebuilt. A cancelled drag charges the refunded
mind cost again instead; a locked view's drag of an item flagged
`0x80000000` is released normally with the answer -1 handed to the
owner's callback.

The view's owner lookup follows the first visible form of its class, and
the player's pack is that view's; the insight check looks for technique 8
in the skill block the view reads and blocks on the right half of the
figure.

The two preview bars at the bottom of the doll are the HUD's own bars at
the doll panel's size: the health bar's yellow and healable shares come
from the character record's two 16-bit words (each × 1/65535), the focus
bar is drawn full, and both are painted by the HUD's painter onto a canvas
covering their frames and drop shadows (`exa_ui::game_gui::paint_preview_bars`).

## Settings options

Behind the Settings page is the hardware configuration unit: the graphics,
audio and input options, their ranges and defaults, the GPU benchmark, and
the settings file. The rows and their behaviour are in `exa_ui::settings`;
the values and pure rules in `exa_formats::hw_config`; the running game's
wiring in `exa_game::hw_config`; the renderer's side in
`exa_render::quality`.

### Ranges, defaults and steps

| row | range | default | slider step |
|---|---|---|---|
| Audio volume | 0–1.5 (0–150 %) | 1.0 | `f × 1.5` |
| Music | on/off | on | checkbox |
| Super sampling | 0, 1, 3, 4 | 0 | `round(3·f)`, one more above 1 |
| Shading quality | 3–6 | 4 | `round(3·f) + 3` |
| Texture detail | 2–3 | 3 | `round(f) + 2` |
| Texture quality | 2–5 | 4 | `round(3·f) + 2` |
| Shadow detail | 4–5 | 4 | `round(f) + 4` |
| Shadow quality | 1–4 | 4 | `round(3·f) + 1` |
| HW antialiasing | 0, 1, 2, 4, 8 samples | 0 | `1 << round(3·f)` |
| Vertical sync | on/off | on | checkbox |
| Show framerate | on/off | off | checkbox |
| Window mode | windowed/full screen | windowed | checkbox |
| Show gameplay tips | on/off | on | checkbox |
| Early checkpoints | on/off | on | checkbox |
| Aim speed | 1–100 shown | 30 (0.29) | `round(99·f) + 1` |

The sliders' labels are the original's, with its leading space: Off, Low,
Medium, High, Very high, Ultra; Coarse, Low, Medium, High, Very high; Off,
Coarse, Low, Medium, High, Very high; and Off, 2x, 4x, 8x for the sample
count. Super sampling has no text at level 2 (the original's second branch
is missing), and texture detail and shadow detail show only Low and High.
The Off branch of the first scale and the Coarse branch of the second
compare unsigned (0–1 and 0–2), so a negative value shows no text.
Rounding is to even, as the processor's conversion does; the conversion is
64-bit and the original keeps its low 32 bits.

A click on a slider goes through the row's step; the value is not clamped to
the row's range beyond what the step produces. A value the step map cannot
name keeps its old label (Exarust's [`Scale::text`] returns `None`).

Window: the original's window-mode checkbox is the inverse of its full
screen flag; setting it and pressing Apply toggles full screen immediately.
The original's full screen is a window over the desktop; it changes the
display mode first only when `ScreenWidth`/`ScreenHeight` ask for a size
other than the desktop's, and keeps the desktop's size ("Display mode not
supported.") when the display refuses it. Exarust gives a borderless window
over the primary display, and exclusive full screen at that video mode when
the settings ask for another size the display lists
(`exa_game::window::fullscreen_mode`). Wayland has no primary monitor, so
there full screen is always borderless.

### Opening the page

The original keeps three copies of the options: the live ones the game
runs with, the page's draft its controls edit, and a saved set that Cancel
and the auto-detection push back into both. Opening the form takes the
saved set from the live options, reads the settings file again over the
live options (each line through its startup rule), copies the result into
the draft, and then puts the live quality levels, super sampling, hardware
antialiasing, vertical sync and window mode back to the saved ones; the
other live options keep what the file gave. Nothing is reset and no
benchmark runs.

The rows show the draft, except the volume (live volume ÷ 1.5), Music (the
live startup value), super sampling and window mode, which show the live
options. Hardware antialiasing is enabled only while the live super
sampling is 0, showing the draft's samples; while super sampling is on the
row is disabled and keeps its position. Moving the super-sampling slider
disables the row above level 0 and enables it at 0.

The volume and texture-detail and texture-quality sliders, and the
vertical-sync, frame-rate, tips and checkpoints checkboxes, act on the
running game at once (each changes only its own live option) as well as the
draft. The super-sampling, shading, shadow-detail, shadow-quality and
antialiasing sliders and the window-mode checkbox change the draft until
Apply. The Music checkbox only mutes the session: the live music value, and
so the file's line and the reopened checkbox, keep the startup value.

Exarust has this model, but the form is not drawn yet:
`HardwareConfig::open` prepares the page (reading the settings file when
the launcher gave one), `rows` gives what the rows show, and the slider,
checkbox, Defaults, Apply and Cancel calls follow the original.

### Apply, Cancel and Defaults

Apply first reads the settings file; when it cannot be read, nothing at
all happens and the form stays open. It then commits the Controls page's
copy of the bindings (when the copy's Forward key is set) and saves them,
and writes these lines, in this order (the first occurrence of each name is
replaced, or the line is appended):

`PerfIndex` (two decimals, trailing zeros kept), `Volume` (percent,
rounded), `Supersample`, `ShadingQuality`, `TextureDetail`,
`TextureQuality`, `ShadowDetail`, `ShadowQuality`, `HWMultisample`,
`InputMode` (`0` for the keyboard, `51` for a controller), `AimSpeed`,
`Fullscreen`, `Music`, `ShowFPS`, `ShowGameTips`, `Checkpoints`,
`VerticalSync`, and `ShowFPS` again (the original's duplicate).

The quality levels, both sampling options, full screen, vertical sync and
the second `ShowFPS` come from the draft; the performance index, volume,
input mode, aim speed, music, the first `ShowFPS`, tips and checkpoints are
the live values, because their controls changed the running game already.
Apply then closes the form, toggles full screen when the window mode
changed, and pushes each of super sampling, shading, shadow quality and
shadow detail that differs from the saved set into the live renderer; any of
those rebuilds the original's shader cache behind a progress screen.
Hardware antialiasing is only stored, so it takes effect at the next start.
Exarust pushes the same options into `exa_render::quality` and the window,
and writes the file itself; without a settings file (a launch with
`--no-settings`) there is no hardware configuration at all.

Cancel pushes the saved set back into the live options (all but the shadow
detail, whose setter writes only the page) and closes the form, dropping the
Controls page's edits.

Defaults acts on the page in view. On Settings it runs the GPU benchmark
(the Deck takes its fixed preset instead: super sampling 3, shading 5,
texture detail 2, texture quality 5, shadow detail 5, shadow quality 3),
makes the result the saved set (every other option copied from the live
ones, except the window mode, which stays the one the page opened with),
pushes it into the live options and the draft (the draft keeps its
antialiasing and window mode; the live options keep their shadow detail) and
refills the rows; nothing is saved. On Controls it puts the shipped keys
into the page's copy, or, in controller mode, the saved aim speed back into
the live one.

The original runs the same auto-detection at start only on a first run
(when `FirstRun` is set), and saves its result then. Exarust does not run it
at launch (its workload and GPU differ, and it would replace the saved
quality); `--benchmark` runs it once, without saving, and Steam Deck
detection is not wired.

### GPU benchmark

The original shows a "Configuration" message, draws sixteen full-screen
quads into a 1024² RGBA8 target from a 512² texture with a simple textured
shader, warms up for half a second with four quads a frame, then takes three
half-second runs of sixteen quads, counting frames. A run's rate is its
frame count over its whole half-seconds; the best rate divided by 60 is the
performance index. The warm-up and runs are on the performance counter, with
vertical sync off.

Exarust measures the same protocol (`exa_formats::hw_config::Benchmark`)
over the same half-second boundaries and score; its clock starts at the
first drawn frame rather than just before it. Its workload is sixteen
full-screen sprites through Bevy's sprite pipeline into the same 1024²
target, not the original's shader: the score is comparable in spirit, not in
number. The auto-detected quality is:

| performance index | super sampling | shading | texture detail | texture quality | shadow detail | shadow quality |
|---|---|---|---|---|---|---|
| under 0.6 | 0 | 3 | 2 | 2 | 4 | 1 |
| 0.6 to 1 | 0 | 3 | 2 | 2 | 4 | 2 |
| 1 to 1.5 | 0 | 3 | 2 | 3 | 4 | 2 |
| 1.5 and up | by display | 5 | 3 | 5 | 5 | 4 |

The top tier's super-sampling level is from
`√(2304000 / (width·height)) × index`: 0 below 3, 1 below 5, 3 below 10,
else 4. A `NaN` index counts as "not below" every threshold (the original's
unordered comparison takes the higher branch), so it lands on level 4.

### What actually takes effect

Exarust applies:

- **Super sampling**: level 0 turns the scene's supersampled target off;
  any level turns it on. Exarust has only the 4× (2× per axis) level; the
  levels above zero are not distinguished.
  A settings file that does not name `Supersample` keeps Exarust's level on
  (live level 4); `Supersample = 0` or `--no-supersample` turns it off.
- **Shadow quality**: the point lights' cube face size and the sun map
  size. Level 4 (the default) keeps the shipped 512 and 4096; 3 gives 256
  and 2048, 2 gives 128 and 1024, 1 gives 64 and 512, 5 gives 1024 and 4096.
  (The exact sizes are a guess; the original's shading path was not
  traced.)
- **HW antialiasing**: Bevy MSAA on the scene camera while super sampling
  is off (1 is off and 2, 4, 8 their sample counts; Bevy has no 1×), read at
  start as the original reads it. Super sampling forces MSAA off, as the
  original disables the control.
- **Window mode, size and vertical sync**: the window's mode (full screen
  borderless, or exclusive for another listed `ScreenWidth`/`ScreenHeight`),
  its size (when windowed and a size was asked for) and its present mode
  (with vertical sync the adaptive `AutoVsync` rather than the original's
  swap interval 1, plain Fifo, which held each late frame to the next blank
  and cost about 4 ms a frame in play; `exa_game::window::ADAPTIVE_VSYNC`).
- **Audio**: the master bus gain is the volume and the music switch mutes
  the music. The original's volume goes to OpenAL rather than to its own
  master gain, which its code never writes.
- **Frame-rate readout**, and the tips/checkpoints/window flags as saved
  options.
- **Controls**: the binding file's table is read and written; the page's
  22 rows (each with a key cell, Power Tree included) show the keys' names;
  a click starts a rebinding of the page's copy and the next key or mouse
  button replaces the slot (Escape cancels); Apply commits the copy. The
  controller-mode switch sets `InputMode` (0 or 51) for Apply and the live
  controller flag, which the running game copies into the saved bindings'
  controller flag; the player then reads the gamepad
  ([PLAYER.md](PLAYER.md), Gamepad). Enable Rumble is a session flag: the
  gamepad reader computes the original's motor speeds, but the original's
  vibration wrapper is a stub that sends nothing, so there is no visible
  effect. Configure Controller is not reproduced (it opens Steam Input's
  binding screen). The key-name rules are
  the original's: the mouse flag's fourth, fifth and sixth codes are
  "Mouse 3/4/5", codes with bit `0x4000` are `<code>`, and other codes are
  the system's key name in title case. The original's codes are scan codes
  (its extended-key fix-up compares the shifted value and never fires);
  Exarust's table holds virtual-key codes, named from a table that gives the
  same names. The middle and extra buttons are named from their saved rows,
  so the middle button is "Mouse 3"; the primary and secondary buttons,
  which the original's page never names, are Exarust's "Left mouse" and
  "Right mouse". A rebinding stores the middle button and the two extra
  buttons as the original does: its window procedure gives them bits 4, 8
  and 0x10, which become the rows `0x8004` ("Mouse 3"), `0x8005` and
  `0x8006` (the slots 4, 5 and 6); the primary and secondary buttons keep 1
  and 2.

Texture quality and texture detail, with the super-sampling level, are
pushed into the texture loader's settings (`exa_assets::texture::
TextureSettings`) whenever the live options change: the quality's filter
mask, anisotropy level and LOD bias base, the detail of the first uploaded
level, and the level the LOD bias follows. The filter mask, anisotropy and
detail are upload-time settings, so textures decoded afterwards take them
and those already decoded keep theirs. The LOD bias is not: the original
sets it on every bind, so a change rewrites every loaded material's bias
(`exa_render::material::set_texture_bias`).

Not reproduced, for lack of the matching feature: shading quality (Exarust
has one shader), shadow detail (no per-detail shadow path), vertex-buffer,
mipmap, swap-copy and GI-precision switches (the GI is already the
original's 8-bit low-precision format), the video-device and video-memory
readouts, and the shader-cache progress screen (Bevy compiles shaders as
needed). The controller bindings are in: the default pad table plays the
game ([PLAYER.md](PLAYER.md), Gamepad); Steam's own binding screen and
prompts are not, since Exarust does not link Steam. The aim speed is stored
and shown but acts nowhere yet: in the original it eases the screen cursor
toward the gamepad's aim point, which Exarust does not reproduce.

## Layers, pickers and the remaining original widgets

The tail of the original's GUI is modelled in plain, tested types; none of
it is wired into the running game yet.

- **Layer** (`exa_ui::gui_task`). A full-screen task form that stretches
  with the root (no attributes, no border; 302 × 402 with no root) owns its
  forms, drawn in list order and hit from the top down: a form whose style
  has the no-hit bit is passed over, and the layer's own hit flag turns
  them all off. Its shortcut table holds key, modifiers and action; each
  entry whose needed modifiers are held scores their count plus one, so an
  unmodified entry fires, the highest score wins and ties keep the earlier
  entry. Shift is never a shortcut and, like an unmatched key, goes to the
  parent unless the layer is the root; a repeat does nothing. The active
  layer and the root draw at 1.0, every other layer at 0.75 and 0.5. The
  frame callbacks run with the layer; one that answers false is dropped.
  The first form made on a task is its main form. Leaving saves the focused
  form when it is on this layer; returning restores it and its control
  while it is visible, else the main form, else the layer, with control −1;
  closing the main form closes the layer, closing the saved form forgets it.
- **Colour picker** (`exa_ui::colour_picker`). One picker per colour-swatch
  control; a second open raises it. A colour swatch (class 0x17) gives its
  caption and colour bytes (× 1/255), kept as the revert source. A refresh
  from RGB turns the colour into bytes (× 255, round to even, the low
  byte), builds six channel-gradient ends (each channel at 0 and 255), sets
  the RGB sliders and their fields (× 255) and the preview, and unless a
  sync is in progress converts to HSV and refreshes the HSV side under the
  guard; it writes the colour back to the swatch as saturated bytes, calls
  the swatch's owner when it has a callback and leaves the word 0x2000. A
  refresh from HSV sets the HSV sliders and fields, converts to RGB unless
  syncing, builds the field's corners `(h,0,v)`, `(h,1,v)`, `(h,s,0)` and
  `(h,s,1)` and refreshes the RGB side under the guard. The field point
  sets hue and saturation; the finish button leaves 0x1000. The conversions
  are the engine's colour helpers.
- **Drop-down list** (`exa_ui::drop_down`). Style 0x98, attributes
  0x4a1470, border 3. Rows are six units; up to sixteen make the list that
  tall plus twice the border; more cut it at sixteen, set the scrollbar
  attribute (never cleared) and scroll by `1 / (count − 16)` worked in
  double. With no control under the pointer a point hits `y / row + 1`,
  held to 1…count. A press arms the current row (5) or asks to close (3);
  the release and 0x100 write the selection back to the owner and close;
  0x4000/0x8000 step the selection. Selecting a row sets the hover and the
  selection. Only the selection draws a band, from y 1 to the client's
  right less the border less one, with a frame a pixel inside; the close
  clears the owner's on and lit bits and frees items the list owns.
- **Palette swatch** (`exa_ui::col_swatch`). A fixed 400 × 112 popup of 32
  columns of 8 three-unit cells, index = column · 8 + row, in the palette's
  lifted colours as bytes; a cell starts a pixel in and ends at the next
  cell's start. The hit answers the cell's control code (index + 1);
  hovering a cell selects it at once; a press selects the cell under it;
  either answers 3 only on a change. A selection stores the index, gives
  the opener's label the cell's colour with the index in its alpha byte and
  the index in hex ("0" for zero, else two upper-case digits) and calls the
  owner (or else the label's own callback). The close, unless kept, and
  0x200 restore the index the popup opened with; 0x100 closes;
  0x1000/0x2000 step a column and 0x4000/0x8000 a row within it. The
  selection is framed in the skin's two frame colours.
- **Creation pickers** (`exa_ui::pickers`). The tone field is 160 × 160, a
  2 × 2 checkerboard of quads over nine colours the original builds at
  start-up (each tone's value lowered by 0.1, × 2.2, raised to 1.5, HSV to
  RGB, × 255). Its colour is the creator's skin tone before encoding (the
  grid in linear light over the base texture, × 0.6); a refresh goes to the
  owner's callback, or tints the character's skin materials (a quarter of
  the way to the pale tone on a material whose level is under 0.1) and
  raises notice 7. The hair field is 160 × 112, three columns blending the
  stored hue run (dark red, blonde, reddish, dark red) repeated in three
  rows, under a shade from black at the top to white at 1.2 times the
  height; the point's colour is the blend at 3u (rounded to even) scaled by
  v·½ + ¼, the marker grey 140·v. The hair picker attaches a control 120
  pixels below itself, as wide and 40 tall, whose value (¼ at first) is
  the hair material's third input; with a material attached a refresh
  re-shades it and raises notice 7. The picker base's pick takes
  `u = x / width` and `v = 1 − (1 − y / height)` in double, held to 0–1,
  ignored within an epsilon of the point; a press over the field answers 7.
- **Skill resources** (`exa_ui::skill_gui`). `sgsklicons.rfi` is cut as the
  origin cell, then five sets of eight 96-pixel cells in two rows of four
  (the origin row techniques 5–8, the row below 1–4; group 1 has none),
  into 8-unit icons sharpened below 96 pixels; `skillicon.rfi` into six
  288-pixel cells as 24-unit icons (blend 0x203, always sharpened); and
  `skillgui.rfi` is rescaled to 44 × 20 units. The once-only initialiser
  builds them and chains the unit-change hook; a unit change rebuilds.
- **Help** (`exa_ui::help_system`). The manual opens `manualkm.fds`, or
  `manualcn.fds` in controller mode, as a 0x2fc × 0x1e6 "MANUAL" page with
  a 0x90-wide topic list (list style, full rows, no icon space) and a text
  box. Each 0x20-byte directory record's 16-byte title is title-cased (a
  lower-case letter after a space or `_` upper-cased) and its word at 0x1c
  is its place in the list; picking a topic loads its block and shows the
  text without its last byte. A tip whose single bit (0–30) is not in the
  settings mask makes a 504-pixel form, its text measured with markup,
  with "Continue" and "Don't show again"; the nine tips are Getting
  Started, Inventory, Matches, Trading, Manage Company, Manage, WARNING,
  Purchased Items and Tournaments, the first two and the eighth preset to
  tick the second control. The call itself does not mark the mask.
- **Trade window** (`exa_game::shop_window`). The page is 0x36c × 0x244 and
  titled "Trade": the character preview (0xe4 × 0x17c) with the player's
  pack under it, the quartered sell basket next to them, the full-price buy
  basket next to the stock, and the stock (0xe4 × 0x234) at the page's
  right edge less 0x10. Each basket re-sums its own total. The bottom row
  shows "Pay:" and the difference, or "Receive:" and its negation; a purse
  short of the payment hides the label, shows 0 and "You cannot afford this
  transaction." and disables the Trade button, which paying with both
  baskets empty also disables. A take-off from the character view goes to
  the pack; a right release hides the window; the close releases the
  preview and ends the trade mode.
