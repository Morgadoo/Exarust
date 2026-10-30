# `exa_ui`

User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started.

So far ([`docs/UI.md`](../../docs/UI.md)): the destination screen's symbol
shader and the company emblem's four-layer shader run per fragment into
images the draw path shows, the divination cards' sheet cells and shadow
mesh draw the same way, and the arena loadout's 27 control records are
decoded (`loadout::CONTROLS`); the game's bitmap font
(`font`), the interface scale from the window's height (`scale`), the
frame-rate readout (`fps`), the loading screen (`loading`, whose camera
draws the whole interface over the game's view, premultiplied), the
character creator's labels and page captions (`creation`; the game crate
owns the rules and places) and its three control-record tables with
their layout and the story backgrounds' names and descriptions
(`creation_forms`), the arena company windows' six control-record tables
(Manage Company 18, Company Records 27, Company Status 12, the loadout form
28, the dismiss and yes/no dialogs 3 each) and their forms' words
(`arena_forms`), a form built from one record array, laid out, rescaled,
placed and answering the pointer through the control messages
(`record_form`), the
in-game world cursor from `cursors.rfi` (`cursor`; the game crate picks
its shape), the window's own pointer (`system_cursor`: the original's twelve-slot
pointer table and show rule, and the menu cursor from `menucursor.rfi`
resampled to the screen-height size, sharpened and set as the window's
custom pointer), the health and focus bars with their fade and the stamina gauge
(`hud`, framed with `guitex.rfi`; `HudShowUntil` holds the bars shown, as
a click on the player's own body does for 5000 ms), the compass's turned
square from `compass.rfi` (`hud_extra`; the game crate shows it while the
player carries a compass and passes the camera's backward axis), the
power marker and skill-ready alert (`hud_status`: the skill-icon atlas's
first cell on a rounded black halo, tinted or outlined at the shared
pulse, rasterised through `gui_draw` into canvases the size of the halo;
the game crate says which shows), the low-health heartbeat's border
(`heartbeat`; the game crate starts it), the fade to black of the
player's fall to death (`fade`, `DeathFade`: 3 s of game time, under the
bars; the game crate starts it), and gamma-space compositing for images
painted on the CPU (`blend`). The skin (`skin`) now also reads a scheme
into its five colour groups with their twelve shades, the accent mixes and
the two alpha ramps, and the constructor's layout metrics. It gathers
geometry through an immediate-mode batch (program, blend, ink, texture
offset, quads/lines/triangles) and draws the flat control parts: outlines,
vertical and alpha-scaled gradients, striped bars, selection bands,
rounded fills and textured frames, edge and bevel frames, solid and two-ink
rectangles, drop shadows and embossed rings, clamped vertical gradients and
the scroll arrows. The control faces, panels, marks, sliders and scroll
bars are built from them in the original's ink order. `gui_draw` submits a
frame: every `GuiLayer` carries a batch, the texture its textured
primitives sample and an optional clip; the renderer walks the layers and
their primitives in submission order and reproduces the skin's four
programs (solid, textured with `TexCol`, rounded with `Scale`, and the
mode a subclass answers when asked: flat for the line skin, the stone for
the HUD's) and the original's blend words (a ten-entry factor table;
0x203 normal, 0x201 additive, 0x103 premultiplied, 0x100 off). The
running game rasterises the frame in sRGB into one window-sized image over
the 3-D view; text and icon quads are layers of their own. A form's chrome
(gutter strips, client fill, double frames), its default background (round
shell, title bar, marks, scrollbars and title blocks) and a page's draw
order (background, clip, scroll, lines, controls, children, shadow) are
modelled too, as are the label placements and the label fade. The menu's
own text program is `SkinMode::MenuText`: `menu_batch` draws its line quads
and `menu_text_fragment` reproduces the shader's red-channel tint, 1.1
gain, tone map and vertex colour; the title sheet (`titlefont.rfi`) is
loaded with the other GUI textures. A draw skin
opened with unit 5 stores spans 10, 5, 10 and 10 and the colour 255, 250,
230, 100 three times. A flat slider at unit 5, place 10, 20 and width 100
stores a thumb
of 5 at value 0 and of 40 at one half, with pad 10. A scroll bar there
with height 80 and lip 4 stores twelve quads and a thumb of 10 at value 0,
or 52 at one half. A frame at unit 5, place 10, 20 and size 100 by 20
stores four corners and two edge bars. Height 80 adds the side bars.
A centred icon of size 16 in a 100 by 80 widget at 10, 20 sits at 52, 52.
A marked state with bit 0x40 sits at 52, 53. A dispatch at unit 5
stores spans 10, 15 and 20, and a spare of 30 at value 1 shifts by 30.
A title of width 40 sits at 30, and style 0x10 draws four corners of span 30,
then edge bars whose reach is 33 at unit 5. A wide form whose style word
has bits 0xc0 equal to 0x80 draws a top from 10, -5 to 90, 5, then a
left side from -5, 10 to 5, 70, then a bottom from 10, 75 to 90, 85, then a right side from 95, 10
to 105, 70. Bit 0x40 of the first flag word then counts one ink copy.
A close mark of -4 then draws a square from 100, 0 to 80, 20.
With the hold bit and without the wide bit, a y of 4 sets mask bit 1
and an x of 4 sets bit 2, then a bar runs from 30, 0 to 70, 10. Bit 0x200 draws a tab
from 16, 30 to 26, 40 when the grip is 10, 20, and a lower tab
from 16, 60 to 26, 70 when the seat is 80, then a quad from 11, 35
to 31, 55 when the grip is 10 and the post is 40. A held form
then draws a left strip from 0, 30 to 10, 50 at unit 5 and height 80,
even when bit 0x200 is clear. Mask bits 1 and 2 together add a cap
from 0, 10 to 10, 30. Mask bits 1 and 2 together then draw a ledge
from 10, 0 to 30, 10, unless bit 1 or the owner's bit 1 is set without
bit 0x800000. A clear hold bit on that band forces mask 15 and a strip
from 10, 40 to 20, 60 at unit 5 and height 100, and a clear bit 1 counts
one call. When the edge bits are not both set, a quad runs from -10, -10
to 30, 30 at unit 5. Mask bits 1 and 4 together, with bit 0x200, draw a
cap from 70, 10 to 100, 30 at unit 5 and width 100. A clear tab bit
draws a narrower cap from 90, 10 to 100, 30, then a shelf from 70, 0
to 90, 10 unless bit 1 or the owner's bit 1 is set without bit 0x800000.
When those mask bits are not both set, bit 0x200 draws a quad from 70, -10
to 110, 30. A clear tab bit draws the same corners with the sign-flip sample,
from 70, -10 to 110, 30. A set tab bit then draws a strip from 80, 30 to 90, 50
at unit 5, width 100 and height 80. A clear hold bit with that tab bit draws
the wider strip, from 70, 40 to 80, 60 at height 100. With mask bit 4 clear,
the plain word's bit 0x200 clear then draws a bar from 96, 20 to 106, 60
at unit 5, width 100 and height 80, then two caps from 96, 0 to 106, 20 and
from 96, 60 to 106, 80. That plain bit, mask bit 4, a clear tab bit, and a
clear hold bit draw none of them. A clear tab bit draws a strip from 90, 30
to 100, 50 at unit 5, width 100 and height 80. A clear hold bit with the tab
bit clear draws the wider strip, from 80, 40 to 90, 60 at height 100. Mask
bits 2 and 8 together then draw a strip from 0, 50 to 10, 70 at unit 5 and
height 80. A set tab bit still draws it. A clear hold bit draws the wider
strip, from 10, 60 to 20, 80 at height 100, then a ledge from 10, 70 to 30, 80.
A set tab bit still draws it. A clear hold bit draws the wider ledge, from
20, 80 to 40, 90 at height 100. When those bits differ, a quad runs from -10, 50
to 30, 90 at unit 5 and height 80. Both bits set draw the ledge instead. A
clear hold bit draws none of that quad. Both arms then draw a bar from 30, 70
to 70, 80 at unit 5, width 100 and height 80. A set tab bit still draws it.
A clear hold bit draws the wider bar, from 40, 80 to 60, 90 at height 100.
Mask bits 4 and 8 together, with the tab bit, then draw a strip from 80, 50
to 100, 70 at unit 5, width 100 and height 80. A missing bit and a clear tab
bit draw none. A clear hold bit with the tab bit draws the wider strip, from
70, 60 to 90, 80 at height 100. A clear tab bit draws a narrower strip from 90, 50
to 100, 70. A set tab bit draws the wider strip instead. A clear hold bit
with a clear tab bit draws the wider clear strip, from 80, 60 to 90, 80 at
height 100. Both tab arms then draw a quad from 70, 70 to 90, 80 at unit 5,
width 100 and height 80. A set tab bit still draws it. A missing bit draws
none. A clear hold bit draws the wider quad, from 60, 80 to 80, 90 at height
100. When mask bits 4 and 8 are not both set, the tab bit draws a quad from
70, 50 to 110, 90 at unit 5, width 100 and height 80. Both bits set draw the
bottom quad instead. A clear tab bit draws the same corners, and a clear
hold bit draws none of either. On that band, a close mark of -4 draws one
square from 80, 0 to 100, 20 at unit 5 and width 100. The earlier square
remains. A mark of 0 or -3, a style that is not that band, and a wide bit
draw none of this square. On that band the dispatcher counts one call
with argument 0. A close mark of -4 still counts one, and the square
remains. A style that is not that band, and a wide bit, count none. Every dispatch then passes the frame to one call, restores
the saved register, sets the stack to the frame, and pops the frame. A held
band, a wide form, and a style that is not that band each do so. The helper then releases one string slot that starts
empty, and an empty slot is not freed. A held band, a wide form, and
a style that is not that band each release it once. The bytes the jump skips
do not run. A span bar at unit 5 from 10, 20 with span 100 and stored word 4
draws a middle from 15, 20 to 105, 28, a left cap from 10, 20 to 15, 28, and
a right cap from 105, 20 to 110, 28. Stored word -3 uses thickness -6.
Shifts 2 and 100 with scales one half and 2 place the first corner at 8.5, 160
and store colour 17 twelve times. An unset view places that corner at positive
zero and negative zero and stores colour 0. Cursor 3 and mark 9 with flag 4
store indexes 3 through 14 and finish at 15. Flag 0 stores none and still
finishes 12 ahead. The square after the bar, with stored word 4 at 10, 20,
draws (3, 29), (3, 13), (19, 13), (19, 29); word 6 draws (-1, 33), (-1, 9),
(23, 9), (23, 33), and word -1 places the first corner at bits 0xdf000000,
0xdf000000. Its cursor starts at 0, so flag 4 and mark 9 store indexes 0
through 3 and finish at 4, and flag 0 stores none and still finishes at 4.
Colour 17 is stored four times. The chip after that square, at unit 5 and
place 10, 20, draws (10, 40), (10, 20), (30, 20), (30, 40) with the sample
0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8. Flag 0x40 binds the second
sample, 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8, and draws the corners
again. Flag 0x10 selects the first ink, mode 0x10000 the second with alpha
255, and any other word the third. A bevel at place 10, 20 of size 100 by 80
fills 12, 22 to 108, 98 as two quads with a hard seam at 60, borders 11.5,
21.5 to 108.5, 98.5 as six lines, and outlines 10.5, 20.5 to 109.5, 99.5 as
four lines. Fill stops are 0.6 and 0.4, border stops 0.65 and 0.35, the left
seam is 60.5 and the right 60. Bit 0x40 selects raised, else bit 0x10 pressed,
else idle. Three begins, twelve binds and three ends are counted. The shipped
inks make raised and idle transparent and pressed black at alpha 0xE0. A
capped span at unit 5 with left 10, right 110 and centre 28 draws a left cap
from 0, 18 to 30, 38, a right cap from 120, 18 to 90, 38, and a middle from
30, 28 to 90, 38. Its cap sample binds twice and the middle once, with twelve
colour writes. A label of width 40, line height 24 and base 18 at 10, 20 of
size 100 by 80 at unit 5 puts x at -35 for style 0x1000, 39 for 0x2000, 65
for 0x3000, 115 for 0x4000, and 13 otherwise; y is 18 for 0x100, 73 for 0x200,
117 for 0x300, 100 for 0x400, and 23 otherwise. State bit 0x10 selects the
first ink, mode 0x10000 the coloured renderer, bits 0x60 the third ink with a
nudge, and any other word the second ink. A rail at unit 5, place 10, 20 of
size 100 by 80 draws a left cap from 0, 85 to 30, 105, a right cap from 120, 85
to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from -5, 30 to
-15, 40. Its cap sample binds twice, the middle once and the thumb once. A
vertical scale of 1 stores heights 80, 104 and 128; one half stores 40, 52 and
64; 2 stores 160, 208 and 0. The rail hit test uses a span of four units, or
six when wide: flags 0x30 equal 0x10 and a y at or above the span return -4
on control bit 2 and -3 on bit 1; otherwise control bit 0x200 and x past the
left limit return -8, -10 or -9 around the limits. The meaning of the codes is
not established. A shade at unit 5, place 10, 20 of size 100 by 80 with side
inset 15 draws cap, bar and cap quads from y 13 to 23 around edges 25 and 95,
the same three from y 97 to 107, and an inner rectangle from 15, 25 to 105, 95.
State band 0x10000 selects the special ink with alpha 255. Opening a skin
leaves the fifth extent at zero; the metric setter called after it stores
10, 5, 10, 10 and 20 at unit 5. With the still bit set, close mark
-9 draws a tab tint from x0 plus one to three units between y0 and y0 plus two
units, -10 draws it below y0, and -8 draws a four-unit square; the tint is
255, 204, 153, 255. Opening the line calls the parent's open once and the
texture load once. The renderer preparation runs two setup calls with a clear
guard and none with it set, returning 1 either way. The ramp alpha scales the
first stored byte by the horizontal scale into 14 entries and the second by the
vertical scale into 48. The parent label aligns like the line label; bit 0x10
draws once in the first ink, mode 0x10000 uses the coloured renderer, bit 0x40
adds a one-pixel drop shadow in the second and third inks, and any other word
uses the fourth. The parent shade draws one rectangle inset by the stored
inset, from 15, 25 to 105, 95 at place 10, 20 of size 100 by 80. The parent
cross draws four line segments around a point; at unit 5 and point 10, 20 its
corners are (8, 15), (13, 15), (8, 25), (13, 25), (6, 18), (6, 23), (16, 18),
(16, 23). The control-code switch returns the close band for -4, -6 and -5 and
the rail squares for -9, -10, -13 and -14; the close band is 17 wide and
`27 − unit` tall, and at unit 5, a width of 100, an inset of 20, a client top
of 7 and a right of 90, code -4 returns 78, 4, 95, 22 (corners, not a size).
Style bits 0x30 and 0xc0 independently select two
double frames in the hot group's shades, the second inset by one; at place 10,
20 of size 100 by 80 they run to 110, 100 and 109, 99. The control image centres
its square at 52, 52, or 52, 53 when marked; mode 0x10000 draws with scale 1,
and otherwise the flags choose the label group's fourth, eleventh or seventh
shade, the styled image forcing the last two opaque.
The inherited bevel takes two booleans, the first winning, else the second, else
idle, and counts under the parent. The parent hit test returns the edge codes
before probing the six small codes around its limits, with -3, -8 and -12 as
fallbacks. The detail dispatch draws one inset rectangle and adds a horizontal
or vertical helper from the style and flag words. The rectangle dispatch draws
one inset rectangle and, on flag 0x40, one cornered overlay at the same
rectangle. The textured strip draws four two-vertex bands between its first and
third floats around a middle y. The slider draws a plain track with a translated
thumb, or a filled inset rectangle with a unit-wide bar. The measure setter
copies three globals and four units plus four into fields 8 through 0x14. The
program setup builds a solid, a textured and a rounded shader, setting `MPrj` to
2 and looking up `Scale` and `TexCol`. The inset bevel adds an inset fill,
border, outline and corner detail on flag 0x40, or one virtual inset rectangle
otherwise. The second slider draws its track at x + 1, y + 2 with its thumb one
pixel lower, or a filled bar. The scroll region clips at the origin plus the
inset and replays its quads, visible children and optional detail. The main
menu quantises its clock to 1/135 and scales it by 1/8. The journal and
character-loot close override closes on a mask carrying both 0x2000 and 0x2. The
class-select screen stores its index and returns 3, playing a sound only when
the index and flag are positive. The skill tooltip draws its icon at two by
five units and its label at x less four units. The transition screen draws
"TO BE CONTINUED" centred on page 2 with blend mode 0x203. The skill selector
clamps its low 16 bits to eight and toggles bits 0x110 on page 1. The inventory
closes on the 0x2002 mask through its callback or the form close. The skill
selector routes its key to the base handler while a positive selection is open
on a page above one, or on word nine with a clear mode byte, and closes with
0x40 otherwise. The end-session menu's click (`dispatch_end_session`) branches on the game
mode (practice 2, story 4, arena 8) and then the clicked item: practice's
victory menu restarts and its other menus run the next fight (the music
fading only into the last), the story's reconstructed menu respawns and its
other items quit, the arena continues or returns to the title; any other
mode runs the first registered handler for it. The click always clears the
clicked and pressed items. `exa_game::end_menu_screen` takes its rows'
actions from it.
The main menu resolves its hover, returns 3 on a change, and rate-limits the
sound to once per 200 units. Its key handler confirms a positive selection,
forwards and backs, and moves within the limit. The inventory detaches by
marking its owner, placing a visible child, releasing its stack and copying the
loot pane's origin, and maps an argument of -1 to page -3. The main-menu name
entry shifts a ten-byte buffer, lowers A-Z and submits on "barechoons". The
splash screen's bounds are the screen and two -1 values, and its destroy
releases, frees two pointers, releases a list and calls the base before the
final free call. The skill selector draws a page-2 title or the measured icon
and label of the selection. A found tooltip row stores the skill index and
threshold, marks the tip, places it from the anchor and stamps the clock. The
loot screen marks its owner and places a visible child when it becomes active,
and the journal copies its origin and assigns its page string. The character
view releases on a free flag, and its clear drops the view and the owner's slot.
The skill selector's show clears bits 0x60 and applies its selection or -1 for
words nine and ten. The class select stores a positive choice, plays its sound
and opens the next form, returning 0x40 for no choice. Its navigation
confirms, forwards and backs, and scans the available-class bitmask for the next
or previous choice. Its hit test maps a point to one of up to four cards laid
out from the clock scale. The main-menu confirm copies the selection to 0x204
and confirms on direction 0x1000; its select sounds the index at most once per
200 units.

The original's widgets ([`docs/UI.md`](../../docs/UI.md), Fonts, Text
boxes, Icons, Tables, Item views, Controls): the six GUI fonts with their
recorded advances and the scale levels that pick unit, fonts and icon
cells (`gui_font`); wrapped and marked-up text, caret moves, the pointer's
caret and the draw (`text_layout`); the multi-line text box with its keys,
clipboard, scroll and drag (`text_box`); the icon atlases and their draws
(`icon_atlas`); the twelve built-in glyphs and the alpha hits
(`gui_glyphs`); the table view (`table_view`), the item view
(`item_view`), the basic controls (`gui_controls`), and how forms and
their controls rescale with the unit, the glyph sheet and the message
box (`gui_rescale`). `GuiLevelState`
(`gui_level`) keeps the level and the atlases with the GUI scale in the
app; the other widgets wait for forms that use them.

The game's own skin is a thin-line subclass of the draw skin. Most of its
methods are modelled in `skin` beside the parent's: the opening (whose
fifth length is four units, set by the parent constructor's call to the
length setter), the vertical scale, the hit test (a title-band point that
misses the corner and the band goes on to the strip test), the slider,
frame, chip and rail, the shade, the four-unit square, the capped and plain
span bars, the label and the widget dispatcher (`dispatch_form`), whose
close-edge tints follow the tab quads they colour. `line_skin` holds the
rest: the five unit lengths and the once-only registration, the line
skin's own centred icon (label shades 4, 8, 7 and 6, opaque or with the
alpha raised by 0x40, and bit 0x40000000 picking the mirrored draw), the
scroll refresh that places a bar window's thumb, the progress strip that
the dispatcher draws for a titled window (split at the title's gap) and an
untitled open bar, the compact skill screen's integer span bar, and the
maximised window's frame, which the dispatcher now draws. At unit 5 a
100-wide window with a 40-wide title splits its strip at 17, and a
maximised 100 by 80 window draws eight quads, its top bar from 30, 0 to
70, 10. None of it is drawn in the running game yet.

## Character view

`character_view` holds the character screen's 3D panel: the `_Inventory`
scene's camera (410 cm from the figure, 30°), its settings (a 100-unit
view, grey 0.5 and 0.75 environment colours, mode 2, light level 1), its
super-sampling choice and background wash (sRGB-decoded or boosted), and
the task's per-frame rules: the race share's easing, the figure turning
its node to the camera's space (`doll_facing`) and the frame counter's
clamped step. It also holds what a pointer and a drag do: the hover pick
over the figure's nodes (the `0x89` and hidden-flag filters, the
settled-item filter, the last node under the pointer, the record hand
entry's or the carried entry's flags), the drag-over answer and the mind cost
a drag off the figure refunds (twice a weapon's desirability, else an
apparel's protection value), the drop onto the figure (apparel worn with
both characters refit and the shown pack charged the bulk at its wear-room
cost, a weapon or shield attached on both and handed to an unlocked shown
pack marked held, the core crystal shown and the shown character
unmarked, and the weapon's cost twice its desirability less what it
displaces), the release on the figure (a scheduled take-out of a slot on
the doll and the shown character, the shown item taken off with its mark
set again, the dragged item taken off with the other pack charged its
bulk, the cancelled drag charging its refunded cost and the locked
flagged drag answering -1), the shown item, the drag start, the owner and player-pack lookups
and the insight check. The two preview bars at the doll panel's bottom are
painted with the HUD's bar painter (`game_gui::paint_preview_bars`), the
health bar from the record's two shares and the focus bar full. The panel
itself is not built yet.

## Arena match board

`match_board` models the arena's "Matches" form beyond the base-form close
and destroy. A board holds the four tabs, the six standard matches, the rank
and tier lists, the six daily events (match, rank, tier and packed coin
entry and prize columns), six tournament winner rows and the season's four
weekly winners. Each tab copies a `MatchRecord` into `Board::selected`,
whose lowest rank, side size, kind and flags drive the lineup: the rank is
the six skill levels halved and capped at Master; a fighter joins when its
rank reaches the floor, the Captain event (type 9) lets teammates be two
ranks lower, at most one leader joins (the player counts) unless flag 1 is
set, a full lineup benches its last fighter, a marked fighter swaps with a
marked teammate, and the company table stays in pool-slot order. The
standard tab scales the standard match's fee and prize by the rank plus one
with the original's coin packing; a rank change moves the tier with it and
the tier rows above the rank are greyed. A weapon event (flag 4) names its
weapon on the tournament tab and checks the first entrant's weapon. The
calendar gives a tournament day every seventh day, a season of four weeks,
the season index `((day + 6) / 28) mod 4`, the reward picture
`seasonrwrd{a..d}.rfi` and the reward order first, third, second, fourth.
The form's rectangles, columns and drawing order are in `layout` and
`Board::paint`, with a 30-pixel row height guessed from the six-row and
eight-row fits; the same layout drives the hit test and a click's tab, row,
cell and confirmation. The form template supplies the board's own controls:
the four tabs (Standard, Specials, Tournament, Season), the bottom row (End
Day, Close, Enter/Begin/Continue, Forfeit, and the status text), the
"Available", "Entered n/m", "Entry:", "Prize:", tournament "Prize" and
"Prize for this season" captions, and the season's picture and item slots,
kept as the template's 23 records (`TEMPLATE_RECORDS`, the board's form
`form_spec`) and laid out by the toolkit's own pass through `record_form`
(x snapped to four, ties to even; four-pixel rows lifted by half the height
less eight; bottom-anchored rows above the client's bottom edge; zero sizes
from the class table). Each control starts at the state the template gives it. The
coin columns carry their callback's decision in the paint list (a dash four
pixels in, or the gold, silver and copper counts). The tournament status
line, primary button, "Entered n/m" label and the show, close and control
states follow the original's tab switch and label routine; the day-end
button follows the day's matches or, on the tournament day with entrants,
whether every entered event has a winner; closing saves the tab and the
three list selections and reopening restores them. `exa_game`'s
`arena_board` fills a board from a career and the generated daily board.

The Settings page (`settings`) now also carries its rows' behaviour beside
the form's records: the row ids and the states they open with (the live
volume, music, super-sampling and window rows, the draft's others), the
slider steps and labels for the graphics, audio and input options, the
anti-aliasing row disabled while super-sampling is on, the checkbox and
slider effects (live, draft or session only), the aim-speed row, the
Controls rows' key cells and binding slots, and the tab/button actions ([`docs/UI.md`](../../docs/UI.md#settings-options)).
The form itself is rebuilt from the original's three record arrays (the
tabs and buttons, 27 Settings records and the 44 Controls label and key-cell
records, read with their styles, radio groups, step counts and callbacks),
laid out at unit 4 by `gui_controls::layout_records` and rescaled with its
two pages by `gui_rescale::rescale_form`, then centred: at unit 4 the
434 × 536 form's tabs sit at 12, 12, Defaults, Apply and Cancel at 40, 156
and 276 on y 484, the Settings page at 0, 64 (426 × 408) and the Controls
page at 0, 56 (418 × 416 over 676 of content). `FormState` hits the
records (the page in view first, edges included, then within two units),
sends them the control classes' press, drag and release messages and turns
their reports into `FormEvent`s (page, Defaults, Apply, Cancel, a slider's
fraction, a checkbox, a key cell's rebinding); the super-sampling slider
sets the anti-aliasing slider to 0 and disables it, the wheel scrolls the
Controls page by quarter steps. The Keyboard and Controller tabs are
modelled too (they need Steam Input, so they stay hidden): their actions
turn the controller mode and view on or off, and with them showing the
Controls page starts 21 units down. The Controls page's power-tree help is
the original's text (`HELP`) at 2, 48 units, its parent's client less 4
units wide and 50 units high (`help_box`), ready for the text box's draw.
The device and memory readouts take the renderer's name or the Deck's and
the video memory (or "Unknown").
The line skin's own control draws are in
`skin` too: each quad's eight-float sample record (`sample_corners`), the
check box and its mark (`check_box_quads`) and the textured batch they go
into (`line_skin_batch`, `push_line_quads`); `line_skin::shown_palette` is
the palette a shown form draws with (alpha ramps 170 and 125) and
`line_control_inks` the skin's disabled, idle and on inks (alpha 80, 104
and 128 times the fade).
Its controller-mode switch feeds the saved bindings' controller flag and the
gamepad reader in `exa_game::gamepad`; Enable Rumble still has no visible
effect because the original's vibration call is a stub. Steam's binding
panel and button prompts are Steam's own and are not drawn ([`docs/PLAYER.md`](../../docs/PLAYER.md#gamepad)).

The forms of the arena and the teleporter (see
[`docs/UI.md`](../../docs/UI.md), Forms), each read from the original's
instructions. The technique selector (`technique_selector`) edits the
character record's six skills (an experience word and five technique
bytes each): a pick into an empty slot replaces the technique in training,
gains the unspent experience up to a level (or `pool · 4 / (4 + Σ levels)`
under the weighted rule, which draws `gain · divisor / 4` from the pool);
a forget closes the slots up, drops the techniques that depend on the
forgotten one and a level for each, and refunds into the technique in
training. Its tree refresh marks learned, assigned, empty, next and locked
slots by the original's ranges. The compact skill screen (`compact_skill`)
is character creation's short tree: its hit word is `row << 8 | column`,
the technique line `unit · (spacing · row + (margin + 16.6))`, the wheel
picks the row nearest (horizontal) or above/below (vertical) the pointer,
pips show the remainder narrowed by `share · 0.8 + 0.1`, nodes stop at the
first empty slot. The arena loadout (`loadout`) reads the form's fields
(arsenal 228 wide at (4, 180), 492 · opacity tall; the equipment 236 wide
at the parent's right edge), the cost rule (a weapon's points less what it
displaces from the hands) and the equip rule (held copies, the form's rank,
the limit, counting the copies the weapon and worn containers hold; the
arsenal's do not count). An item dragged back onto the arsenal
(`return_to_stock`) is taken back by an arsenal entry of the form's
category lent under the swap mask, sorted into the kit container it left
(`sort_drop`: weapon, off-hand copy or worn), and the copies the kit still
holds are counted for the rank's record; an off-hand copy frees the worn
entry a hand holds. `apply_drop` is the list model's equip onto the kit
behind the equip gate. Its paint frame lays the backdrop, the two
containers and the preview out at the constructor's rectangles as flat
fills, in the paint's order. Its list model of the kit is Exarust's own.
The constructor's per-mode record changes are `mode_records` (used by
`controls_for`): mode word 1 widens Previous and Next, hides Dismiss, the
rank, points and encumbrance rows and the shelf and box buttons, and shows
the hire block (HIRE, fee, upkeep, profession); mode 3 disables the list,
the rank and four shelf buttons and hides Previous, Next, Dismiss and the
points row. Every refresh marks the arsenal entries the shown kit has in
hand or wears with `EQUIPPED_MARK` (`mark_equipped`). The divination
reading (`div_card`) deals three distinct cards out of twenty-two, eases
the deal and the turns by `(3 − 2t)·t²`, turns each card half over about
a swinging axis and draws the original's drop shadow; the side facing
away is culled as the renderer's backface culling does, so a card shows its
back until the turn passes edge-on and its face after. The emblem editor
(`emblem_form`) starts on palette 0xd3, 0x72, 0x5a, 0x4e and rolls with
`state · 0x8088405 + 1`. The destination screen (`selection`) places its
nodes by the engine's sine and cosine (`engine_math`, forwarded from
`exa_physics::engine_math` so there is one implementation), eases the ring
twice and the alpha once, tries three orbit nodes, and its journey glows
the teleporter, charges a small crystal, changes level when the resting
bubble is told to shrink and, for the adapter, marks the teleporter and
saves when the bubble is freed.

The skill tree's rules are in `skill`: the learning row's requirement mask
(the screen's node-present table, the record's prerequisite table, both
prerequisites funded), a node's press state, the screen's own unlearning
(with its committed points) and choosing, the experience pass and ready
flag, the ring, satellites and link search with the engine's sine across,
the group icons, orbit bars and spokes, the hover tooltips, learning row,
forget dialog and popup fade, and the screen's fades. The record's
prerequisite table and surplus query come from `exa_character`. Its
opening keeps the content alpha and the opening scale apart (the hide
scales the scale), its open/toggle plan builds the atlases, starts the
reveal sound, suspends the controls and shows the form, and its close
restores them. `skill_gui` holds the two shader sources, their uniforms
and the fixed `MPrj` projection matrix with the reveal sound, and `skill_draw` builds
the screen's vertices: the page background, the ring page's dim, the node
plates and five spokes, the node and level icons, the orbit bars and the
tooltip, learning-row and forget-dialog boxes and text anchors. The
extended controls in `controls` model each class's message rules: the text
field's and combo box's edit state and key dispatch, the drop-down
placement, the colour drag, the spinner's commit, press, key filter, drag
step, auto-repeat capture and buttons, the progress, separator, image,
texture, caption, trackbar and scroll-bar geometry and draw calls, with
`CVTSS2SI`'s indefinite result and the unordered clamps that let a NaN
through. The screen and the controls are not drawn by a form yet.

The thaumaturgy power tree's drawing is `tree_draw`: the six immediate
programs and the spline renderer's procedural ribbon, the empty slot, icon,
frame, glow, flame, link glow and new/picked rings, the bar's icons, marks,
flames, glows and empty slots with its opening and closing animation, the
creation prompt and the tooltip's caption and wrapped description (with the
interface level's main font: its advances, glyph quad and line step, sampled
from the level's atlas when installed or the base font's atlas scaled to the
quad), the label mode and the dragged power's preview. Their texts take
the skin's text ink (`skin_text_ink`, the line skin's seventh label ink) and
sit on the skin's shaded box (`skin_box`: the rounded program in black,
grown by `unit · style · 4` pixels, style 4); label mode draws every box
before any name. A prompt button's
caption sits one unit past its centring and, focused, is the skin's glowing
text (`skin::glow_text` with the line skin's inks). A domain whose icon
sheet is not installed draws no icons (the original assumes every sheet
loads); a sheet's cells per side are its pixel size over 512. The link
ribbon follows the spline renderer: the two vertices a
sample, the start and end handles one tangent step beyond, the row and side
coordinates, the join term and the source-to-end dim alpha. The ring passes
sample the flamelets sheet and the glows, flames and preview sample the frame
sheet, as the original binds them. The links' texture unit is unbound and
the spline program samples nothing: the ribbon is wholly procedural.

`wheel_draw` is the domain wheel child's own paint, in the original's
order: the flamelets backdrop over the 280-pixel form (`par0` six times the
400-second phase, `par1` the reciprocal of the eased focus reach, or 1000
at or below 0.01), the six sector bands from `round(0.64R)` to
`round(1.2R)` (`R = 140·scale·s`, `s = (2 − shown)·shown`) with their phase,
open and share fall-off, the chosen and fading choice's glow, the six icons
on the 76-pixel ring (the hot icon's four glow copies 1.5 pixels off each
diagonal, the chosen icon in its domain colour, the rest in the line skin's
third ink), the ring frame turned by `iπ/3` in its second ink, and the
label's rounded box and name. Its three programs' sources are kept and
their fragments are functions (`back_fragment`, `glow_fragment`,
`sector_fragment`, `rounded_fragment`); they draw with blend `0x103` over
the repeating flamelets sheet. Places round to whole pixels once the show is
above 0.99999, compared in double.

## Form base and the in-game unit

`form` extends its metrics with the original form class's own rules: the
constructor's defaults (the 0.25 float is the vertical scroll step), the
unset and show places in the parent's client, the sibling layers read
from the attribute word (the child raise searching from the second child
and taking owned popups along), the active-form test, maximise and
restore, the runtime resize, the 256-tick fade (whose eased mode sets no
alpha), both scrollbars and their setters, the wheel, held arrows and
thumb drags, the point conversions rounded with `CVTSS2SI`, the hit test
(client, children, then records exactly and within two units), hover,
focus, caret, Tab, the base form's key handler (`form_key`: Tab among
the records, the focused record's answer, else the parent's key slot
unless the key repeats or was typed), the key/press/drag routes, the frame codes a press,
grip drag and release act on, the snap slots and the records' rescale,
the maximise and restore steps with the window-state hook's words, the
close cascade over owned popups and children, and the control table's
copy. A shown form refreshes the client areas of its whole tree, parent
before children; and freeing a form sweeps its tree, parent first,
through the GUI's single-form slots (`sweep_freed_form`): most slots that
held it clear, the task's pressed slot with its press word, while the
main form's focus fallback, the focused form's parent focus or cleared
focus (by the parent's visibility), the tooltip reset and the drop
callback reset come back as ordered follow-ups.

`game_gui` is the in-game unit's non-drawing half: the screens' tags
(0xce1c2010 inventory, 0xce1c2040 loot window, 0xce1c2030 journal), the
eight-slot open-container registry, the inventory, loot and journal show
rules and places, the journal's entries, the power slot's, counter's and
bar control's messages, blocks and draw places, the status marker with its
glow radius, share, tint and hide clock, skill-ready alert, level curtain
and its hold-count reset, compass, heartbeat sound fade, menu cursor and
HUD draw order, and the save and close scopes. The loot window is "Body",
244 × 440 at x 400 with an unset y, refused for a flag word whose second
byte is 3 and matched to its body. The inventory's preview writes its model
values 0.15 and 0.05, the scene and draw words, the appearance copy and the
build delay, its build callback's encumbrance dots (ten times the body's
encumbrance, written into the inventory's rating-dots record),
and its preview bars' fills and frames. The
bars and their frames are `hud`'s; their show test (`bars_shown`: a
changed or NaN health, a dropped focus, the power bar, combat mode or the
unsigned hold clock) and the gauge's (`gauge_shown`: a shield in combat
mode) are here and the HUD draw calls them every frame, with the power
bar off because Exarust has none. The counter's and bar control's message
handlers, the inventory fill's page states and plan, the journal's list
state, the technique selector's open plan and the deferred fill's answer
(retry until the generator has the node) are tested rules with no live
caller yet.
`exa_game::gui_screens` opens the inventory and journal from the I and J
hotkeys on a `GuiTask` and draws them through `gui_draw` and
`exa_game::gui_paint`: the frame, the inventory's pack panel and its item
icons when the item manager's atlas has any, its Encumbrance label and
Primary button, and the journal's New and Erase buttons and the selected
entry's text, from the constructors' words and records; a dead body's loot
window draws its frame and its pack and equipment panels. They draw with
`line_skin::shown_palette` at their fade and the line skin's button frame
(`skin::draw_frame` in `line_control_inks`). The item panel's
icons use `SkinMode::HltCol`, the item GUI's own program (below). The
character view, preview bars, the loot panels' body items and the journal's
entry list are not wired.

## The item panel's icon program

`SkinMode::HltCol` is the item panel's subset program: it samples the icon
texture, sums eight neighbouring alphas in the shader's order, scales the
sum by the vertex's green and three quarters, mixes the highlight
(1, 0.75, 0.45) towards white by the vertex's red, lays that over the pixel
by the square of its transparency, adds the glow to the alpha and scales
the fragment by the vertex alpha (`item_icon_fragment`, pinned by tests).
The item GUI's five control classes are `gui_controls::ITEM_CLASSES` and
register after the shipped seven.

## Scope

- GUI skin and icon atlases from `Resource.rpk` (`guiskin`, `guitex`, icon sheets).
- Manual pages (`.fds`) use a small BBCode-like markup (`[algn=2][tcol=FFE399]...`).
- Inventory with item icons rendered from 3D models, as in the original.

## GUI core

`gui_core` is the window manager: a form is anything implementing
`FormHost` (the original's virtual methods: hit test, select, press,
release, drag move, drop, close), and `GuiTask` keeps the task's state —
the focused form and control with its tooltip request and the tick that
shows it, the modal form, the pointer's target, press point, held word,
double click, drag threshold and capture, the drop hand-over and the cursor
shape. `PlainForm` is a basic record over `gui_controls::dispatch`;
`message_box_plan` builds the original's Ok/Cancel box, `place_form`,
`clamp_form` and `edge_rect` its placement, clamp and grips, `GuiMetrics`
the level switch's lengths, and `set_control_on` a toggle group's switch.

`gui_text` holds the text support routines: measuring with a font's advance
table, selecting a font and its texture, one line's glyph quads (plain,
centred, a span, truncated with the ellipsis, or the fitted tail), a wrapped
block hanging from its line tops, the line count, the width fit, and the
menu font's record (`titlefont.rfi`, rescaled half to even). `text_edit` is
the single-line editor: the four-byte view (anchor, caret, the window's end
and its first byte), caret from x, the horizontal scroll, the key and
navigation handlers, clipboard copy/cut/paste, the caret and the draw's
three paths, and the markup tags. `viewport` keeps the 3D viewport
control's supersampling levels, jitter table and samples, camera scale,
stereo eye, overlays, frame-rate counter, pixel/space conversions and its
cursor actions.

## Layers, pickers, swatches and help

`gui_task` is the top-level layer (`TGUITask`): a full-screen form that
stretches with the root (no attributes, no border), its forms drawn in
order and hit from the top down (the style's no-hit bit and the layer's own
hit flag skip them), its 16-byte shortcut table (each entry scores its
held modifiers plus one, so an unmodified entry fires; Shift is never a
shortcut and is forwarded to the parent unless the layer is the root), the
active/root 1.0 draw against the inactive 0.75 and 0.5, the notice word,
the frame callback list (a callback answering false is dropped) and the
focus hooks: leaving saves the focused form when it is on the layer,
returning restores it while visible, else the layer's main form, else the
layer; closing the main form closes the layer. `colour_picker` is the colour
picker logic: one picker per swatch (raised on a second open), the opening
colour kept for the revert button, RGB and HSV sliders and their fields,
the six channel-gradient ends, the four field corners, each refresh
updating the other form once under the sync guard, and the write-back as
bytes to a colour swatch with its owner callback; the conversions are the
engine's (`exa_formats::palette::{rgb_to_hsv, hsv}`). `drop_down` is the
combo box's popup: style 0x98, attributes 0x4a1470, border 3, six-unit
rows cut at sixteen with the scrollbar bit set (and never cleared), the
step in double, the hit clamp, a press arming the current row, the
release and 0x100 writing back the selection, the selection band and its
text. `col_swatch` is the palette popup: a fixed 400 × 112 form of 32
columns of 8 three-unit cells in the palette's lifted colours, hovering
selecting at once, the hit answering the cell's control code, the close
and cancel restoring the opening index, the step words, and the label's
colour (index in its alpha byte) and engine hex text.

`pickers` holds the character-creation fields: the tone picker's 2 × 2
checker over the nine start-up tone bytes and its colour, the skin tone
before encoding (`exa_assets::human_skin::tone_linear`), handed to the
owner's callback or tinting the skin; the hair picker's three columns
over the stored hue run, its value scale, grey marker, four-vertex shade,
attached control and set slot; and the picker base's pick (double
divisions, flipped v, epsilon test). The hair material's shading routine
is not here (the character generator ports it). `skill_gui` builds the
two skill atlases (the origin cell first, five sets cut as two rows of
four with the origin row techniques 5–8, 8-unit icons sharpened below 96
pixels; six 24-unit single icons) and the 44 × 20-unit background, with
the once-only pass; the page constructor and the record attach are the
skill screen's (`SkillScreen::open`, `SkillScreen::attach`).
`help_system` is the manual (`manualkm.fds`, or `manualcn.fds` for a
controller; topic records whose word is the list position, titles
title-cased, a page's text without its last byte) and the one-time tips
(single bits 0–30 of the settings mask, the 504-pixel popup measured with
markup, "Continue" and "Don't show again", the nine titles and their
presets).

## Table, skin and blend details

`table_view` now keeps the column choice list that a press opens (its
entries, width, anchor and picked entry) and routes every key that
is not Enter, Escape or the empty-edit Delete through `text_edit` (typing,
Backspace/Delete, arrows, Home/End, Ctrl+C/X/V, horizontal scroll), so an
edited cell behaves like the original's field. A press inside the edit
puts the caret and anchor together; a drag moves only the caret and scrolls
by the first column's width. The draw reports each header's inset
rectangle, the owner-drawn cells from the first visible row, the open
field's text state and the shadow below the header (the client's height
less six units, negative when the client is shorter); the table's default
selection is the first row and its centre is half the client's width and
height. Hovering a row answers 3; no row keeps the last hovered row.

`skin` carries the two traced hooks of the abstract base (`SkinHooks`),
the GUI globals (lazily made default skin, active skin prepared once per
skin, the alpha/fade pair with its unclamped alpha and rounded byte), the
`guiskin.rfi` atlas at 40 by 20 units with the textured frame's slots, the
texture-bind `TexCol` rule and the textured skin's vertex and fragment
sources. The control parts go through the skin's own slots: a panel's plain
arm, the control box, the control shell and the slider's thumb draw the
raised box (its gradient, bevel and outline in the control group's shades),
the scroll bar's trough is the accent gradient with a raised-box thumb and
its style frames run first on the moved rectangle, and the shell frames its
own rect before that box. The form background's two scroll troughs and its
horizontal thumb are raised boxes; the vertical thumb's idle line gradient
and its pressed bevel run between the control group's seventh and fifth
shades. A mark or trough is pressed while the button is still down on the
stored pressed code (the code under the mouse only makes a mark hot), and
the hovered mark box's inner bevel is one shade brighter than the outer.
The panel, like the shell, draws its style frames on its own rect first. The selection band reads the highlight group's
fourth or fifth shade and its seventh or eighth by the selected state, and
the form hit test answers the close, large and small mark codes (-4, -5, -6)
in that order and the horizontal scrollbar's two arrows before its fallback.
The form draw's temporary string release is compiler-generated string
cleanup; Rust ownership covers it, so it has no item. Part of it is called by the running UI through `gui_paint`; the rest waits for a form.

`blend::UiBlendCache` is the GUI blend-state switch with the original's
one-word cache: a word equal to the last does nothing; the word `0x100`
under the mask `0x3f0f` turns blending off; any other word turns it on (when
the last word was the disabling one) and sets the source factor from bits
8 to 11 and the destination from bits 0 to 3 of the recorded ten-entry
table. Nibbles past the table read zero, and nibble 12 reads the cache's
own last word, as the original's table lookup runs on into the data after
it. Nothing in the running UI calls it; Bevy's blend states replace it.

## Retail screens and popups

The remaining retail forms are modelled from their own routines.
`tooltips` holds the delayed show (256-unit fade, 2500-unit auto-hide),
the box sizing and placement (text plus four units, two units taller than
a line, kept on the screen, 153 alpha) and the skill tooltip's 50-unit
field. `modal_screens` holds the message box, the shader warm-up screen's
three phases, its 200-unit fades and its one-row-a-frame walk of the
renderer's 29-row warm-up, and the key-capture screen's warning, binding
table, Escape close and mouse codes; both screens' paints produce their
centred text quads in the palette colour under the fade. The running
game's rebinding and Apply drive them (`exa_game::hw_config`), and
`exa_game::settings_form` draws them. `transition` is the state machine between game states
(600/800/300/2000-unit fades, the switch, the hold and the fade back).
`class_select` lays out the four story portraits and their chosen card,
and has the picker's hit test and key steps. `dialogue_choices` is the
reply list's entry measure, hit test, pick, fading colours, rows and
layout: the height target, the 30/17-unit minimum, the narrator's immediate
height and the 500-unit animation from the height being drawn. `speech` is
the speech-bubble frame: the width fit at 320, the placement's projection
and margins, the rounded box of radius 3 at alpha 128 (the narrator's grown
by four units) and the wrapped white text at alpha 160. `dialogue_window`
syncs the conversation engine's reply panel into that list: each entry is
measured at the panel's width less eight units, the `[Other]` toggle is
appended unmeasured and the height is laid out once per change, so a
running animation is not restarted. It also makes the form on the first
talk (400 × 120, centred, 152 pixels above the bottom), keeps it shown
while the engine's panel is open (an emptied list leaves the box), holds
the engine's pick for a second, hovers the bands under the pointer, answers
a click with the hovered entry and builds a frame's draw (the eased height,
the rounded box of radius 2 at alpha 160, "N." and the wrapped text in the
label or highlight ink).
`picker_fields` has the creator's marker (its point and its free or
bound colour), keys and physique cross, the physique picker's
constructor, the point's body-shape weights and the press, release and
change steps.
`popups` has the item card, the container popup (its close saves the panel's contents into the container record; `GameScreen::Container` is its screen kind, tagged `0xce1c2013`, closed by a right click) and the parchment (its size, alpha hit test with the factor divided in double precision, and its draw: one textured white quad at the GUI's alpha, the fade step first and the alpha scales reset after a finished fade). `dialogs` has the colour window's
gradients and hue table and the confirmation dialogs. `help_screen`, `task_stack`,
`game_tips` (its eighteen tips and six indexed messages), `colour_swatch`, `view_helpers` (the viewport's super-sample
and cursor-action list) and the journal close in `game_gui` complete the
unit. The draw records are plain data; no new form is painted by the
running app yet.

The developer editor's controls ([`docs/UI.md`](../../docs/UI.md), Dormant
editor controls) are reproduced but never created, as in the retail game:
`popup_menu` (the popup menu with its shortcut labels, submenu chain and
popup placement), `tree_view` (the tree view with its pre-order rows,
chevrons, rename field, drag and popup mode) and `object_editor` (the object
appearance editor's control handlers and palette placement). `skin`'s
`PaletteInk` names the palette shade a draw tints with.
