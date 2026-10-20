# Proposal

## Why

The player's keys were hardcoded in the steering and combat systems, and the
original's 23-slot binding table and its saved file were unmodelled. Cursor
Look (slot 9, V) was missing entirely: the mouse could not turn the facing,
capture the pointer, or bring the camera behind it.

## What Changes

- Model the 23-slot virtual-key table, its shift/alt/control either-side
  reading, and the saved file (version 101, controller flag, count, keys;
  older files without the flag).
- Read the table for movement, primary attack, alternate attack, combat
  toggle, orbit camera and interact.
- Implement Cursor Look: holding V captures the mouse, hides the system and
  world cursors, turns the facing by 3.6 × Δx / window width radians per
  pixel, turns the camera's target behind the new facing, and restores the
  pointer where it was on release.
- Keep the shipped V slot; the settings template's SPACE label is recorded
  as the template, not the default.

## Capabilities

### New Capabilities

- `input-bindings`: the 23-slot table, either-side modifiers, and its saved
  file.

### Modified Capabilities

- `player-movement`: Cursor Look capture and the table-driven controls.

## Impact

- `exa_game::bindings`: the table, codec and Bevy key mapping.
- `exa_game::player`: movement, attacks, orbit and combat read the table;
  a new `CursorLook` resource and system; `player_combat` leaves the aim
  alone while captured.
- `exa_ui::CursorCapture`: the world cursor and system pointer stay hidden
  while Cursor Look owns the mouse.
- `docs/PLAYER.md` (Bindings, Cursor Look) and `docs/UI.md` (World cursor).
- Evidence: `context/re/player-control.md` §1.1 and §1.3; the binding file
  and table addresses in `context/re/component-map/sections/ui.md` §1.1.
- Out of scope: the settings screen's edit/rebind UI, `TReadKeyScreen`,
  controller axes and the full 23-key name table.

## Phase

Phase 3, player behaviour. Bindings and Cursor Look.
