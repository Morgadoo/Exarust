# Proposal

## Why

The Settings and Controls pages are visible in the retail game and were missing from Exarust. The retail 1600×1080 screenshots give their exact rows, values, tabs, control bindings and footer buttons.

## What Changes

- The Settings page: device/memory info, the audio slider, seven quality choices, and five toggles across three rows.
- The Controls page: fourteen label/key bindings.
- The two tab rectangles, content rectangle, row tops, label/control/value columns and the Defaults/Apply/Cancel footer.
- The layout is scaled linearly from the retail 630×800 form; the binary's embedded control-template records are not decoded, so the layout is documented as screenshot-derived.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the Settings and Controls pages.

## Impact

- `exa_ui::settings`, from `Tab`, `Rect`, `CHOICES`, `SettingKind`, `SettingItem`, `SETTINGS`, `ControlItem`, `CONTROLS`, `SettingsLayout` and `layout`.
- `docs/PLAN.md`; the coverage headline is unchanged because the page content is screenshot-derived rather than a decoded function.
- Evidence: the user-provided retail Settings and Controls screenshots at 1600×1080. The control-template records around `0x1002b2b00` and the label strings at `0x100308928` are the binary source for a later exact pass. No new corpus files.
- Out of scope: the exact template records, the scroll behavior of the controls list, and the settings apply/reset logic.

## Phase

Phase 7, UI. The Settings and Controls pages.
