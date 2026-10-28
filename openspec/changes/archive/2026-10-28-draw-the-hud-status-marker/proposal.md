# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the game GUI paragraph (the
loot window, power slots and HUD of the gamegui lanes): the HUD's power
marker and skill-ready alert had their places, pulse and conditions but
were never drawn; the compass's interface fade was always 0; the form
class's close cascade, maximise/restore callbacks and control-table copy
were left out, and the HUD's message layer was only named.

## What Changes

- `exa_ui::hud_status` draws the marker and the alert: the skill-icon
  atlas's first cell (96 px at the origin of `sgsklicons.rfi`, resampled
  to 8 units) on the skin's rounded black halo (the two-unit square 3..5
  units in, grown 8 units, `round(0.6 · pulse · 255)`); the marker's icon
  tinted (140, 160, 255) at `CVTSS2SI(pulse · 255)`, the alert's outlined
  at the pulse in the skin's seventh label and third highlight inks.
  Each is rasterised through the GUI draw path into a canvas the size of
  its halo.
- `exa_ui::game_gui::StatusClock`: one pulse clock, stamped by the
  alert's first frame, read as it is by the marker, cleared only by the
  HUD switched off.
- `exa_game::hud` shows the marker while the player's caster knows
  powers and none is a new pick, and takes the compass's fade from the
  open power tree's window fade.
- `exa_ui::form`: the close cascade (controller step, closing flag,
  active child closing its task, owned popups, children, hide, callback,
  redraw), the maximise and restore steps with the window-state hook's
  words 2 and 0, and the control table's copy.
- `exa_ui::hud_status::message_layers`: the HUD's one-line message and
  its shadow, tested (the shipped game never sets its flag).

## Evidence

The status routine (962 bytes) draws the halo through the skin's rounded
fill with radius 8 and alpha `0.6 · ease` (double, narrowed), selects the
textured program, sets the colour (140, 160, 255, `CVTSS2SI(ease · 255)`)
— the decompiler drops the alpha argument — and calls the icon atlas's
plain draw; the alert calls its outlined draw, whose inks are the current skin's label and highlight entries. The
handle is the atlas's first add, the 96-pixel crop at the origin of
`sgsklicons.rfi`. The ready-alert stamp is cleared only on the HUD-off
branch of the HUD draw. The compass reads a word only the power tree's
draw writes (its window fade, 0 at its end). The close (645 bytes) never
removes a form from a list; the task's detach clears only its active and
remembered pointers. The message flag is read and never written.

## Out of scope

Frame dragging of the live screens (the skin's frame hit codes are not
ported, so no title code reaches the screens), the power bar on the HUD
(the power tree's), the dialogue layer (the dialogue unit's), the icon
atlas's sharpening of reduced icons, and a live skill record for the
alert.
