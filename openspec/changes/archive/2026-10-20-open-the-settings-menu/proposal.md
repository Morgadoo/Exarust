# Proposal

## Why

The end-session menu opens the settings form (`0x100124240`): it creates and attaches the form when none exists, otherwise re-shows it, lays it out, selects entry 1 behind a resume flag and finishes with −1. Two small helpers close an existing form (`0x1001242f0`) and clear its flag (`0x100124320`). Exarust routes the end-session pages but not the settings form's lifecycle.

## What Changes

- Opening creates the form when none exists, or re-shows an existing one.
- Opening lays the form out, selects entry 1 behind the flag, and finishes with −1.
- Closing and flag clearing act only on an existing form.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the settings menu's lifecycle.

## Impact

- `exa_ui::menu`, from `open_settings_menu`, `close_settings_menu` and `reset_settings_menu`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: no published percentage moves; `TEndSessionMenu` is not in the largest-components table. The headline stays 14.4% and 5.7%.
- Evidence: `0x100124240`: the null check on the global settings pointer, the create call `0x100128220` with class `0x312ed0`, the attach pair `0x100029470` and `0x100029660` against the re-show pair `0x10002c8f0` and `0x10002ae50`, the layout `0x10022c740`, the flag byte `0x49a300` selecting `0x6c` = 1, and the finish `0x100029aa0` with −1. `0x1001242f0`: the close call `0x100128d30` on an existing pointer. `0x100124320`: the byte at `0x218` cleared on an existing pointer. No new corpus files.
- Out of scope: the form's draw and the resume handler's call list.

## Phase

Phase 4, UI. The settings menu's lifecycle.
