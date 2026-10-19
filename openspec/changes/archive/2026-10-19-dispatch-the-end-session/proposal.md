# Proposal

## Why

The end-session menu resolves a global page and a local action into a form, a game call or a registered handler, then clears its action fields. Exarust does not model that dispatch.

## What Changes

- Pages 2, 4 and 8 have built-in actions 1 and 2.
- Page 8's first action depends on whether the form page is 2 and whether the last-round flag matches.
- Any other page calls a registered handler whose key equals the page, or does nothing.
- The action and field 0x204 are always cleared and the method returns 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the end-session menu.

## Impact

- `exa_ui::menu`, from `dispatch_end_session`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 477-byte method's page switch, five form constructors and handler table at 0x4f0110. No new corpus files.
- Out of scope: the forms and calls themselves.

## Phase

Phase 7, the interface. The menu dispatch.
