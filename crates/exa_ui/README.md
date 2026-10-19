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
times. The slider, the frame, and the widget dispatcher are not in.

## Scope

- GUI skin and icon atlases from `Resource.rpk` (`guiskin`, `guitex`, icon sheets).
- Manual pages (`.fds`) use a small BBCode-like markup (`[algn=2][tcol=FFE399]...`).
- Inventory with item icons rendered from 3D models, as in the original.
