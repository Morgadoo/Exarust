# `exa_ui`

User interface: main menu, character creation, inventory, arena management, HUD, manual pages, tooltips, cursor.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: started.

So far ([`docs/UI.md`](../../docs/UI.md)): the game's bitmap font
(`font`), the interface scale from the window's height (`scale`), the
frame-rate readout (`fps`), the loading screen (`loading`, whose camera
draws the whole interface over the game's view, premultiplied), the
in-game world cursor from `cursors.rfi` (`cursor`; the game crate picks
its shape), the health and focus bars with their fade and the stamina gauge
(`hud`, framed with `guitex.rfi`), the low-health heartbeat's border
(`heartbeat`; the game crate starts it), the fade to black of the
player's fall to death (`fade`, `DeathFade`: 3 s of game time, under the
bars; the game crate starts it), and gamma-space compositing for images
painted on the CPU (`blend`). A draw skin (`skin`) opened with unit 5
stores spans 10, 5, 10 and 10 and the colour 255, 250, 230, 100 three
times. A flat slider at unit 5, place 10, 20 and width 100 stores a thumb
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
empty, and an empty slot is not freed. A held band, a wide form, and a
style that is not that band each release it once. The bytes the jump skips
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
size 100 by 80 draws a left cap from 0, 85 to 30, 105, a right cap from 100, 85
to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from 105, 30 to
95, 40. Its cap sample binds twice, the middle once and the thumb once. A
vertical scale of 1 stores heights 80, 104 and 128; one half stores 40, 52 and
64; 2 stores 160, 208 and 0. The rail hit test uses a span of four units, or
six when wide: flags 0x30 equal 0x10 and a y at or above the span return -4
on control bit 2 and -3 on bit 1; otherwise control bit 0x200 and x past the
left limit return -8, -10 or -9 around the limits. The meaning of the codes is
not established. A shade at unit 5, place 10, 20 of size 100 by 80 with side
inset 15 draws cap, bar and cap quads from y 13 to 23 around edges 25 and 95,
the same three from y 97 to 107, and an inner rectangle from 15, 25 to 105, 95.
State band 0x10000 selects the special ink with alpha 255. Opening a skin at
unit 5 also stores a fifth extent of 20. With the still bit set, close mark
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
the rail squares for -9, -10, -13 and -14; at unit 5 and trim 2, code -4 at
x 100 returns 74, 4, 95, 26. Style bits 0x30 and 0xc0 independently select two
textured frames, the second inset by one; at place 10, 20 of size 100 by 80 they
run to 110, 100 and 109, 99. The control image centres its square at 52, 52, or
52, 53 when marked; mode 0x10000 draws with scale 1, and otherwise the flags
choose slot 0xd8, 0xf4 or 0xe4, with the style adding a colour to the last two.
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
five units and its label at x less four units.

## Scope

- GUI skin and icon atlases from `Resource.rpk` (`guiskin`, `guitex`, icon sheets).
- Manual pages (`.fds`) use a small BBCode-like markup (`[algn=2][tcol=FFE399]...`).
- Inventory with item icons rendered from 3D models, as in the original.
