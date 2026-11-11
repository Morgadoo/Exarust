# Proposal

## Why

The in-game GUI unit's screen sweeps and its save of the open screens were
modelled but never called:

- entering combat mode (Tab, or a script's combat-mode call) left the
  inventory, journal, loot and container windows open; the original closes
  every in-game screen and the power tree;
- Control Mind's handover and hand-back closed only the power tree; the
  original first closes the inventory, container, journal and loot family;
- opening the journal (J) left combat mode on; the original's opener turns
  it off first;
- a game save wrote the level section without the open container popups'
  contents or the journal's text; the original saves the open screens first.

Phase: Phase 6 interface milestones of docs/PLAN.md (game GUI). This change
does not edit PLAN.md.

## Evidence

REA (Ghidra on `Exanima.exe`), checked against the disassembly:

- the unit's close-every-screen routine (tag mask 0xce100000, active task
  then root task, back to front, then the power screen's sweep with the
  caller's bar bit) and its callers (the combat toggle's entering branch,
  the session ends, the arena);
- the container-family close (mask 0xce1c0000, the active task front to
  back, then the sweep with the bar bit set) and its Control Mind callers;
- the journal opener's first block (combat mode off through the script
  setter after the out-of-action and dead returns) and its text box's Tab
  callback (combat mode on);
- the save of the open screens (per tag: container panel save, inventory
  and loot panel saves when loaded, journal store) and its callers (the
  slot saves of the save-games unit and the main menu).

## What changes

- `exa_game::gui_screens`: `close_game_screens`, the container-family close
  by form, `ScreenSweep` and `sweep_screens` (screens, then
  `power_screen::sweep_forms`), `save_open_screens` and the
  `save_open_screens_for_saves` system in the save chain.
- `exa_game::hotkeys`: Tab's entry sweeps every screen; J turns combat
  mode off first. `exa_game::flow`: `CombatMode(true)` entering sweeps.
  `exa_game::mind::runtime`: the handover and hand-back sweep the family.
- `exa_ui::game_gui`: `journal_leaves_combat`.

## Out of scope

- The journal text box's Tab callback: Exarust's journal text is drawn
  read-only and takes no keys, so nothing can call it yet.
- Other forms in the 0xce10 family (the arena manage window, tag
  0xce10ae20) are not swept; the arena's own callers of the sweep.
- The inventory and loot panels' saves: their items already live in the
  bodies' possessions.
