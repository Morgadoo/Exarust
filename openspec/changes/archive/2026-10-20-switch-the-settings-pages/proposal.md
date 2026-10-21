# Proposal

## Why

The Settings and Controls children are switched by page and the Controls list scrolls; those two behaviours were not modelled.

## What Changes

- Page 0 shows the Settings child at (0,64,426,408); page 1 shows the Controls child at y 56 (widescreen) or 96, height 472−y.
- The Controls scrollbar over the 412-px client and 676-px content: 15-px rail, 9-px minimum thumb, `232`-px thumb and `17 + 150·f` thumb y at unit 4.
- Both scale by `unit/4`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the settings page switch and scrollbar.

## Impact

- `exa_ui::settings`, from `PageSwitch`, `switch_page` and `controls_scroll`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGUIForm` becomes **16%** in the source rating. The headline rises slightly.
- Evidence: the settings form's page switch and the generic scroll with the controls child's 676-px content. No new corpus files.
- Out of scope: the wheel handler, the Keyboard/Controller sub-tab bodies and the apply/reset logic.

## Phase

Phase 7, UI. The settings page switch.
