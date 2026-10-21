# Proposal

## Why

Showing the skill screen marks it for redraw and stamps a deadline; leaving it either calls the base, waits out the page and its transition, or runs the clear call. Exarust has the node position only.

## What Changes

- Showing runs the relayout when the screen's open flag is set, sets bit 1 of the state and stores the clock plus the delay.
- Leaving calls the base when the closing flag is set; a non-zero page does nothing.
- With bit 0 set and a deadline more than 400 above the clock, leaving does nothing; with bit 1 set it does nothing; otherwise it runs the clear call.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's show and leave.

## Impact

- `exa_ui::skill`, from `show_skill_ui` and `leave_skill_ui`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 1% to 2%.
- Evidence: the 63-byte show and the 116-byte leave: the open bit `0x20` of the style word and its relayout call, the state field, the two deadlines, the closing field, the page, the 400-unit subtraction, the game clock and the clear call. The delay global is zero in the file and is a parameter. No new corpus files.
- Out of scope: the relayout, base and clear routines.

## Phase

Phase 4, UI. The skill screen's show and leave.
