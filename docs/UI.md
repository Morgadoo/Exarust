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

The hub's company windows are modelled as rules (`exa_game::manage`); no
hub screen draws them yet. Their controls are read from the original's form
tables.

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
- **Match board** (`exa_ui::match_board`): the board's constructor, tab
  switch, list refreshes and fighter assignment.
- **Skill screen** (`exa_ui::skill`): `TSkillUI`'s 186-byte node placement
  at `0x100224570`; the requirements, states, ring icons, tooltips and
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
  while both a save and a checkpoint exist. The rules (gear
  filter, starting skills, experience budget, kit tables, page flow, save
  choice) are plain functions in the game crate with tests pinned to
  concrete numbers. The form itself, its model preview, sound playback and
  the power tree are not drawn yet.
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

### The main menu screen

The running app opens the main menu with F1 (`exa_game::menu_screen`); the
original opens it from its session flow, which belongs to the game modes.
The screen follows `TMainMenu`'s rules (`exa_game::menu_pages`):

- **Pages.** Title: BEGIN, CONTINUE (greyed without a story save), ARENA,
  MANUAL, SETTINGS, EXIT. Arena: NEW CAREER, CONTINUE (greyed without an
  arena save), PRACTICE, VERSUS (only with the versus lobby), BACK.
  Practice: NOVICE, EXPERT (greyed until it is unlocked), BACK. In-game:
  CONTINUE (NEW GAME in practice), MANUAL, SETTINGS, EXIT. The end pages:
  CONTINUE, SETTINGS, FORFEIT (or EXIT). The row origins are 0.38, 0.42
  with five arena rows (0.45 with four), 0.45 and 0.39 of the height.
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
  out.
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
the screen only performs its own page changes, the resume, the exits to the
title and the quit.

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

## Skin

The base draw skin (`exa_ui::skin`) is the original's `TGUIDrawSkin`: it
owns the palette, the layout metrics and the GUI batches, and draws every
flat-coloured control part. The textured subclasses (the HUD's `guitex.rfi` stone skin, the
`guiskin.rfi` atlas) add their texture binds and shaders; the flat
primitives below are the base class's.

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
  shows four groups at a 72-unit radius. A slot sits at angle
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
  over 256 ms.

## Item windows

The item GUI's rules, read by REA from the original's item panel, icon
cache, inventory, arsenal and item-card code. None of these windows is
drawn yet: the rules wait for the forms that will show them.

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
  apparel rows above 0.25; coverage always. The striking side's shown rows
  (slash, crush, pierce) fill the card's rows from the first. A
  description's pronoun marker names `HeShe`, `HimHer`, `HisHer` or
  `HisHers`, giving he/she, him/her, his/her or his/hers by the owner's
  sex bit; the description box sits `0xb4` pixels below the page's top.
- **Inventory** (`exa_items::inventory_window`): 244 × 644 units (628
  compact, 590 inside the arsenal), the doll above a 228 × 180 pack of
  capacity 400 and items up to 50; worn items count against the pack.
  A right-button release closes it.
- **Arsenal** (`exa_items::arsenal`): up to five 236-unit columns, 60 units
  apart, scrolled by two arrows; borrowed items stay in their column
  marked `0x200` until returned.
- **Icons** (`exa_items::icon_cache`, `exa_items::item_gui`): one icon per
  item and variant (lit models by their condition byte), counted by
  reference; 40 units square; framed at 70 (30 for apparel, 20 close),
  the weapon light preset for weapons, and given a one-pixel alpha halo
  (the highlighted one also lifts green). Cards show rating dots (whole
  up to the value less a quarter, then a half dot), a slider, coin groups
  and crystal sockets; a popup opens centred on the cursor, its bottom 8
  units below it. The four socket sheets (`dotrating.rfi`,
  `coinicons.rfi`, `wpnflpgrpico.rfi`, `crystalicons.rfi`) keep their
  three, three, three and six cells. A socket row hovers by fifths of a
  unit, takes only crystals 431, 451, 452 and 499, shows a crystal's card
  on a right click and starts its drag on a drag, and hands the previous
  crystal back when one replaces it; a finished drag clears the socket's
  kind but leaves its charge byte. Committing the row writes the sockets
  into the item's weapon section (refreshing its electric effect), else
  its apparel section (refreshing the player's abilities), and an AR
  crystal fires the game's fourth function.

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

A drag over the figure is accepted only when its item section allows it:
apparel needs a race that wears it (bit 4) and a view that allows
changes; a withheld weapon or shield needs the matching body permission
(`0x10000`, or `0x20000` for a weapon's second bit); a race whose core
word takes the AL crystal accepts item 500 while nothing is shown. A drop
wears apparel, takes a weapon into a hand, takes a shield into the off
hand, or takes the body's core crystal. A hover shows the item under the
pointer and its icon, and clears the shown item when the icon cannot be
resolved. A frozen view starts drags only from a held item. A release
takes a held item out of its hand (the pack drops its held entry), takes
the shown item off when the shown icon is the dragged one, and otherwise
takes the dragged item off and charges the pack its bulk. The view's
owner lookup follows the first visible form of its class, and the
player's pack is that view's; the insight check looks for technique 8 in
the skill block the view reads and blocks on the right half of the
figure.

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
  same names. Exarust's own mouse codes 1, 2 and 4 are shown as "Left
  mouse", "Right mouse" and "Middle mouse".

Not reproduced, for lack of the matching feature: shading quality (Exarust
has one shader), texture quality and texture detail (the texture loader's
`exa_assets::texture::TextureSettings` follows both rules but keeps the
retail user's values; the live options are not pushed into it yet), shadow
detail (no per-detail shadow path), vertex-buffer,
mipmap, swap-copy and GI-precision switches (the GI is already the
original's 8-bit low-precision format), the video-device and video-memory
readouts, and the shader-cache progress screen (Bevy compiles shaders as
needed). The controller bindings are in: the default pad table plays the
game ([PLAYER.md](PLAYER.md), Gamepad); Steam's own binding screen and
prompts are not, since Exarust does not link Steam. The aim speed is stored
and shown but acts nowhere yet: in the original it eases the screen cursor
toward the gamepad's aim point, which Exarust does not reproduce.
