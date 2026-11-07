# Proposal

## Why

docs/PLAN.md Phase 1 (the engine's platform and GUI layer), the osgui gap
lane: TMessageBox (rated 0%), TCursor (15%), RF_GUI_3DViewport (0%) and
RF_OS_Functions (68%). The message box's layout, key handler and button
answers were modelled but no box was ever shown: the hub's hire refusals and
the company dialog's empty-name refusal only logged their text.

Evidence (REA, Ghidra listing and assembly of `Exanima.exe`): the
message-box routine and its five callers (the first-run benchmark's
"Configuration" box, the creator's "Character Name", the recruitment's and
the hire's refusals, the company dialog's "Company Name"); the box class's
two handlers; the line skin's constructor (its border, 2); the "Ok" literal;
the GUI unit's start-up, which builds the cursor object (its constructor
stores the position unknown on both axes and the drag distance 2, then the
shape is set hidden) and the pointer update that compares the read position
with the stored one; the 3D viewport's action-list add and remove.

## What Changes

- The hub shows the original's message box: a form built from the box's
  layout (text width in the small font plus 64, at least 320, plus twice the
  skin border; 104 tall; label; Ok/Cancel), centred across and placed down
  from its design-pixel top, on top of the hub's form stack. Enter and Space
  close it, Escape closes it with -1, Ok stores 1 and Cancel 0
  (`exa_ui::modal_screens::open_message_box`, `exa_game::hub_forms`).
- Its live callers: a refused hire ("Hire": insufficient funds, no room,
  merchant, physician or trainer already employed) and the company
  dialog's NEXT with an empty name ("Company Name"). `--open-hub-window
  nameless` runs the latter for scripted shots.
- The GUI pointer starts with its position unknown (`i32::MIN` on both
  axes), so the first read always counts as a move, as the original's
  cursor constructor sets it.
- The viewport's cursor-action list removes by the handler's code (the
  method pointer's first word), not by its object, per the assembly.

## Out of scope / findings

- The creator's "Character Name" box belongs to the character creator's
  owner; the recruitment box follows the creator, which the hub does not
  run; the first-run benchmark's "Configuration" box has no counterpart
  (Exarust runs no start-up fill-rate benchmark).
- The box's close-mark handler (masks 0x100 and 0x600) stays a model: the
  hub forms draw no live close mark.
- The cursor-action list has no live caller: Exarust's player controls,
  navigation, character view and inventory read the pointer in their own
  systems. The super-sampling get and set are served by the live settings
  (`HardwareConfig::live.supersample`, read by the texture LOD bias and the
  shader sample counts, applied on Apply), the renderer honouring only on or
  off.
- RF_OS_Functions: no new routine; the routine table records what is live,
  replaced by winit/wgpu, or Windows-only.

## Impact

`exa_ui` (modal_screens, gui_core, view_helpers), `exa_game` (hub_forms,
option_forms, arena_hub_live, staff, emblem); docs/UI.md; exa_ui and
exa_game READMEs.
