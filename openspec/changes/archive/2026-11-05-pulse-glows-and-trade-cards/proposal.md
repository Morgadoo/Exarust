# Proposal

## Why

Wiring backlog E7 (script glow pulses), the last part of P6 (the trade
window's description phrases) and the draw order between the item card and
the hub windows that the round 52 useeffects lane found. The glow task, the
crystal drain and the description phrase builder were ported without a live
caller; the trade window offered no card for its items, so no phrase was
ever shown there; and the hub windows and the card had no order between
them, so the card could draw under the trade window.

Phase: Phase 4/7 item, story and arena milestones of docs/PLAN.md (no
PLAN.md edit in this change).

Evidence (REA, Ghidra on `Exanima.exe`, checked against the disassembly):
the game unit's object use, its pulse task creation and frame routine, the
crystal drain, the collect pass, the socket and inventory drains and the
game tick's once-a-second call; the level load's end (collect) and its task
list clear; the item GUI's card builder, its card form constructor
(attribute word) and the panel's card request; the trade window's
constructor (its attribute word and its panels' accept masks) and its stock
load (the flags each stock entry is loaded with); the form base
constructor's default attribute word. Data: `exa-tool mechanisms` over the
base tier and every story level (one usable class 2 kind 3 object:
"PedestalHand", placed in `exanima67`).

## What Changes

- `exa_game::glow_pulse` (new): the pulse tasks on the live scene, the
  emissive rescale of the object's mesh parts and its first-part child
  meshes, the child voxel lights' intensity, and the level load's cancel.
- `exa_game::object_click`: a scripted device's use (kind 3) pulses the
  object and plays its usable section's sound at 0.3; `--object-click` takes
  `#id` to aim at a placed object.
- `exa_game::crystal_drain` (new): the collect pass at the end of a level
  load over the player's hands, worn items and carried list; the
  once-a-second drain of the socket and inventory sources.
- `exa_assets::live_objects`: `write_section_byte`, a logged byte write.
- `exa_game::hub_windows` / `arena_hub_live`: a right press over a stock or
  pack row asks for the item's card with the flags its panel holds it with
  (`0x80000000` for the shop's own items); the release after it does not
  close the window; the scripted purchase's card opens over the window.
- `exa_game::item_card`: the card form's attribute words; the card draws
  after the hub windows.

## Out of scope

- Running the scripted object's own script (the use's first step).
- The container crystal sources (kinds 1 and 2), filed by callers outside
  the game unit.
- Item icons and basket rows in the trade window, and the trade window's
  row labels, which draw about one row above their hit rows (a paint
  stand-in that predates this change).
- The plain card without a description, which is on the normal layer.
