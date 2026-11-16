## Why

Roadmap task B4 (lane `uifix`): four GUI findings of the evidence2 lane.
F565: the saves' character screen drew none of its records. F564: the
respawn's rebuilt player held Exarust's carried torch, which its stripped
record does not hold, and stood at the level's own start, outside the
level's rooms, until the step placed it. F561: the arena hub's hire page
drew its profession line over the points line, its coin values a row above
their captions, and no candidate in the preview. F562: the inventory's
character view cuts a golem's head off.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- The loadout form's constructor in its hire mode (1) rewrites its record
  array before the layout: Previous 0x70 wide, Next at x 0x78 and 0x70
  wide, Dismiss, the five rank records and the ten shelf and box buttons
  hidden (0x110), the six hire records shown (0); it builds a text box
  0xec x 0x100 at y 0x20, four pixels from the form's right edge, and
  leaves the arsenal box out. The record view writes the profession's
  description into that box and builds the candidate's body for the
  form's character view.
- The coin control draws its counts on its own baseline, the record's
  middle with the main font's centring height.
- The respawn spawns the player anew from the stripped record at the
  respawn device.
- The character view's camera is fixed by its constructor (target
  (0, 100, 0), 410 cm towards (-200, 160, 360), 30 degrees) and its
  camera controller's record (pitch held at 0.15 rad, distances 10 to
  1410, no wheel handler); binding a character does not move it, so a
  larger body is framed as a human's (F562: as the original).

## What changes

- `exa_game::save_screen`: the saves' paint runs after the menu's systems
  and the game screens' draw, which clears the GUI queue each frame
  (F565).
- `exa_game::save_live::RebuiltPlayer`, `exa_game::torch::spawn_carries_torch`
  and the viewer's spawn: the respawn's rebuild gets no carried torch and
  starts at the respawn placement (F564).
- `exa_ui::arena_forms::HIRE_LOADOUT_FORM`, the hub's loadout open, the
  coin control's paint and the kit's hire view (F561).
