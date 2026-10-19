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
then edge bars whose reach is 33 at unit 5. The drawing after those bars
is not in.

## Scope

- GUI skin and icon atlases from `Resource.rpk` (`guiskin`, `guitex`, icon sheets).
- Manual pages (`.fds`) use a small BBCode-like markup (`[algn=2][tcol=FFE399]...`).
- Inventory with item icons rendered from 3D models, as in the original.
