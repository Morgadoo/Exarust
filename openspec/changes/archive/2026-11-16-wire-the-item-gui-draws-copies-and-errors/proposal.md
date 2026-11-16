## Why

Roadmap tasks B4 and B1 (lane `itemgui2`), the item container
(TGUIItemContainer), the item GUI (SG_ItemGUI) and the procedural
materials (FLX_Materials_Procedural), after the first item GUI lane
(round 224). The panels' place pass, a body's equipment scatter and the
card's sentences drew from generators of their own; a drop never made a
local copy of a marked base item; the socket row had no drag-over answer;
an item without a model still got an icon slot; and an unknown procedural
material id was not reported.

## Evidence

REA (Exanima.exe), checked against the assembly where constants or order
matter:

- The place pass, the worn panel's per-item entry and the description
  sentence builders step the engine's one generator word
  (x · 0x8088405 + 1, the high half of the product for a draw below n).
  The worn panel first stores 0xBA28 in the run-time library's seed.
- The drop from a panel other than the linked one hands the dragged item's
  id to the object database's instancer with the level-local tier: an
  untagged base record whose header word has bit 0 is cloned there, and a
  changed id acquires a new icon with the payload's flags below the top
  four bits. The entry keeps the payload's flags.
- The socket row answers the drag-over event of an item payload with its
  crystal test (the four power crystals); only then does the drop reach it.
- The streamed icon acquire answers 0 for a record without a model
  section; the list load skips an entry whose resolve fails.
- The procedural program lookup reports "Invalid Procedural Material ID."
  at level 4 under "Error" for an id outside 1 to 25, builds ClothBasic's
  program for it and keeps it in the id's slot.

## What Changes

- The item GUI's draws on the frame (place pass, equipment scatter, card
  sentences) step the frame's engine generator, the one the frame's
  effects draw from; the scatter's seed restarts the frame's run-time
  stream.
- A drop into an item panel from another panel makes the local copy, which
  the destination and the possession record take.
- The socket row's drop is gated by its drag-over answer.
- The panel acquire gives no slot to an id the loaded catalogue holds no
  model for, and the list loads leave such an entry out.
- The painter notes unknown material ids and the engine log shows the
  error once per id.

## Impact

`exa_game` (panel_live, item_panel_draw, doll_view, socket_drag,
shop_panels, hub_windows, arena_hub_live, item_card, body_inventory,
gui_screens, engine_log, thaum::live), `exa_assets::paint`,
`exa_formats::{objdb, procedural}`. GUI-side only: the simulation trace
does not change.
