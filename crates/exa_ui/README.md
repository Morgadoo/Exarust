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
one call. The helper body and the sign flip are not in.

## Scope

- GUI skin and icon atlases from `Resource.rpk` (`guiskin`, `guitex`, icon sheets).
- Manual pages (`.fds`) use a small BBCode-like markup (`[algn=2][tcol=FFE399]...`).
- Inventory with item icons rendered from 3D models, as in the original.
