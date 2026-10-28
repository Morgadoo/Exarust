# Proposal

## Why

The game-main unit's second pass (lane gamemain2) extended `exa_game::flow`
and `menu_pages` with the menu rows each page builds, the story wake-up's
whole animator write, the story start's record writes, the session begin,
continue and leave bookkeeping, the end-menu exit and the unit's menu text
shader. The review against the assembly found the story start's extra
"roles" were really gear (a carried entry and an off-hand item), a flag
named after the help screen that belongs to the end menu, a title-logo flag
read as the help flag, an invented GUI-scale step and wake-state resets the
original does not make.

Phase 7 (Game modes and content), the game-mode flow milestone.

## What Changes

- `menu_pages`: the title's CONTINUE reads the story's save manager, the
  start page's the arena's; the in-game page reads NEW GAME in practice; the
  arena pause page's last row reads EXIT once the session's value-4 bit is
  set; the pause pages never grey CONTINUE out.
- `flow`: the story start clears the role list, adds 7, 1 and the
  background's own role, gives the unknown background carried item 0x10c and
  the thaumaturge's power word and value, and gives the knight and proctor
  item 0x1f in the off hand and in the carried list (flags 0x100); the
  get-up's hold and finish writes and the revival's; the expert unlock reads
  bit 0; the boot order; the practice start's role 0x10, controls and motion
  3; the session begin, continue and leave; the end-menu EXIT; the level
  exit's fade bit; the settings keys' value kinds; the unit's finalisation.
- `menu_text`: the unit's menu text fragment program and its colour rule.

## Evidence

The game-main unit's page builder, end-menu builder, story start, practice
start and next fight, story hook, revival, session begin/continue/leave,
back-to-title, level exit, settings registration and finalisation, read in
the assembly; the carried-list append (item, flags, then the 0x28/4
half-words) in the character-data unit; the menus' shared row hit test for
the end menu's flag; the main menu's draw for the title logo flag.

## Out of scope

- Applying the flow's steps in the running game (they stay logged); only
  the page state and F1's hold of the dialogue and hook are live.
- Drawing the end menu, the menu text shader in the row painter, the
  practice-won end menu's no-row-selected case, the arena pause callbacks.
