# Proposal

## Why

A first pass reproduced the original's form base class and the in-game GUI
unit's routines from the pseudo-code. A review against the instruction
listings found rules that were misread or invented: the sibling layers read
from the style word instead of the attribute word, the child raise's search
start, an active-form test with a visibility check the original does not
make, unset places measured from the wrong edges, maximise and restore
fields, flooring where the original truncates, truncation where it rounds,
a hit test that skipped the client check and the exact pass, a wrong close
band, a horizontal wheel turned the other way, the constructor's vertical
scroll step taken for a wheel step, an eased fade that the original
discards, snap slots without the container's bounds, the inventory and loot
tags swapped, the player's inventory placed where only other characters'
go, a guessed journal key mask, guessed sizes and tags, identity
placeholders and copies of the HUD's layout. The running game opened the
screens from a second key reader that looked at every character's
condition.

## What Changes

- The form base: constructor defaults, unset and show places, the sibling
  layers and raises, the active test, maximise and restore, the runtime
  resize, the fade, both scrollbars, the wheel and held arrows, thumb and
  grip drags, the point conversions, the hit test, hover, focus, caret,
  Tab, the key/press/drag routes, the press and release frame codes, the
  snap slots and the records' rescale.
- The in-game unit: the screens' tags, sizes and places, the show rules,
  the journal's entries, the power slot's and counter's messages and
  blocks, the status marker, skill-ready alert, level curtain, compass,
  heartbeat sound fade, menu cursor, HUD draw order, the save and close
  scopes and the technique selector's opening rows.
- The HUD bars keep the values their show rule compares with only in a
  frame that shows them.
- The running game: the I and J hotkeys send a screen request through the
  hotkey handler; the inventory and journal open on a window manager task.
  The launcher's `--press` reads Esc.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the form base class, the in-game GUI unit and the
  HUD bars' show rule.

## Impact

- Phase 7 (game modes and content) of `docs/PLAN.md`: the forms the in-game
  screens are built on and the screens the player opens.
- `exa_ui::{form, game_gui, hud}`, `exa_game::{gui_screens, hotkeys}`,
  `exarust`; `docs/UI.md` and both crates' READMEs.
- Evidence: REA (Ghidra 12.1.4) on `Exanima.exe` 0.9.5 — the pseudo-code
  and instruction listings of the form class and the in-game GUI unit, the
  inventory's constructor, the journal's save, the six-skill level sum and
  the `%` the counter appends, read from the binary.

## Out of scope

- Drawing the forms, their controls, the status marker, the curtain and the
  compass; the 3-D character preview's scene work; attaching the item
  panels to the inventory and loot windows; the loot window's constructor.
- The skills key's screen, the power tree and the power bar.
- The task stack's activation hook and the forms' tooltip and gamepad
  questions, which belong to other units.
