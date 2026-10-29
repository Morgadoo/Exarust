# Proposal

## Why

docs/PLAN.md, Phase 4 (the player's controls and behaviour), the playerctl8
lane (SG_PlayerControls unit routines, TPlayerBehaviour). The item cursor's
placement markers had a tested rule but nothing registered them and lifted
props carried no object id, so nothing snapped; the lift placed an item
wherever the cursor went and its release dispatch followed an invented GUI
reading of the hover word. In combat the pad was turned into keys, and a
lone forward step ignored the target.

## What Changes

- The level loader registers the original's placement markers (model nodes
  named `[…`) and keeps each placed object's template; the lift keeps the
  placed object's id and template, gathers the markers each frame and
  snaps the held node onto a matching marker's frame.
- A lift frame holds only when the box fits and the item is in reach (a
  marker only in reach); a failed frame restores the last placement that
  held and counts as out of reach.
- The lift's release follows the original: in reach put down; cancelled,
  last fitting placement, left on the marker (frozen for an `L` marker) or
  put back. A fall ends a lift as a cancelled one; a press thaws a frozen
  object. The forced release sends a cloth drag to the wheel's handler.
- In combat the pad's stick vector is the footwork's analog word.
- A lone forward word's goal bends at the target's 60·scale cm circle.

## Evidence

REA (Ghidra) on Exanima.exe 0.9.5: the sector attach's marker list, the
lift frame and its release, the forced release, the behaviour's word
reader and movement goal and the approach bend, read in the instructions;
corpus scan of the installed packs for `[`-named nodes (three shipped
marker nodes) and volume flags (none at `0x22`).

## Out of scope

Doors and wheels as bodies under the hand, the GUI item layer, the
deferred interaction list's trigger, the space-free test at a marker, a
drop handed to a GUI window.
