# Proposal

## Why

Reviews left three ported rules of the player controls and the game flow
without a caller in the live game (Phase 4 — Combat and items, and the
Phase 3 game flow, see docs/PLAN.md): the use cursor's begin and end, the
deferred GUI drop's run, and the story scripts' `GameEvent` handler, whose
background unlock never reached the settings store. The last review also
found that the light toggle read the record's lit bit, which a lit torch
dropped onto the doll no longer carries, so the first toggle after such a
drop turned the light the wrong way.

Evidence: REA (Ghidra) reads of the controls' start-up (the use hook it
installs), the use hook and its cursor callback, the world view's event
handler that calls a pending callback with the end event, the item panel's
mouse handler and its double-click class, the use dispatcher, the GUI drag
claim and deferred drop, the game-main unit's `GameEvent` registration and
handler with its settings write, and the light toggle, the weapon attach
job and the flip step.

## What Changes

- `exa_game::player_controls`: `panel_use_class` (the panel's double-click
  class), `UseCursor` (the live state between begin and end); `use_end`'s
  dispatch gate is the desk word's bit 0 (a guess: the left button).
- `exa_game::use_cursor` (new): the item panels' double click calls the use
  hook; the hook lifts the cursor; the next press of any button ends it and
  is taken from the hand, the panels and the card; the dispatcher picks under
  the pointer and the door unlock is applied; `--use-item` debug flag.
- `exa_game::gui_drop` (new): a pack drag held over the world is claimed,
  run on the next frame through `run_spawn`, placed where the cursor's ray
  meets the level and lifted onto the item cursor once its body exists
  (`hand::lift_gui_drops`); `--gui-drop` debug flag.
- `exa_game::flow`: `Flow::game_event` and the `run_game_events` consumer of
  the scripts' `GameEvent` calls; the background unlock writes its word.
- `exa_items::possession::HeldMark` and `Possessions::burning`: the light
  toggle reads the held part's mark; the doll's take-in sets it from the
  item's full flags; `--doll-drop` debug flag.

## Out of scope

- The dispatcher's pour, clean, repair and GUI branches' effects, the
  rattle's sound, and the readable's window (`UseReleased` reports them).
- The player's effects of the `GameEvent` handler (fragments, awakenings,
  lines, sounds).
- A drop out of a loot or container window, and of an entry held in a hand.
- The flip's own animation turning the held part's mark (Exarust turns it at
  the toggle).
