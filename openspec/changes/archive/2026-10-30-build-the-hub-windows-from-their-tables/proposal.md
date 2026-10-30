# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: round 37's
arena pass left the hub's company windows painted by hand. Manage Company's
buttons and coins, Company Records, Company Status, the loadout form and the
dismiss and yes/no dialogs were not built from the original's control
tables, and their buttons did nothing.

## What Changes

- The six constructors' 0x68-byte record arrays (Manage Company 18, Company
  Records 27, Company Status 12, the loadout form 28, the dismiss and yes/no
  dialogs 3 each) and their forms' words in `exa_ui::arena_forms`.
- A general record form (`exa_ui::record_form`): laid out at unit 4,
  rescaled with its child panes to the live unit, placed by its placement
  bits, the pointer through the control messages.
- The hub draws these windows through the GUI paint path and routes their
  callbacks to the arena's rules (`exa_game::hub_forms`).
- `--open-hub-window <name>` opens one for scripted screenshots.
- The loadout form's duplicated 27-record table is replaced by the 28-record
  array.

## Evidence

REA/Ghidra and the retail image: the constructors of Manage Company,
Company Records, Company Status, the loadout form and the two dialogs, the
form's table builder (the last index it takes), the arrays' type records
(their element counts), the base form constructor's border, placement and
attribute words, and the manage refresh's writes to the coin, Recruit, Hire
and End Day records.
