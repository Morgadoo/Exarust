# Proposal

## Why

The end-session menu opens the settings form: it creates and attaches the form when none exists, otherwise re-shows it, lays it out, selects entry 1 behind a resume flag and finishes with −1. Two small helpers close an existing form and clear its flag. Exarust routes the end-session pages but not the settings form's lifecycle.

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
- Evidence: the settings open: the null check on the global settings pointer, the create call with the form's class, the attach pair against the re-show pair, the layout, the resume flag byte selecting entry 1, and the finish with −1. The close helper: the close call on an existing pointer. The flag helper: the flag byte cleared on an existing pointer. No new corpus files.
- Out of scope: the form's draw and the resume handler's call list.

## Phase

Phase 4, UI. The settings menu's lifecycle.
