# Proposal

## Why

The skill tooltip draws an icon from a table row and a label beneath it. Exarust does not model the tooltip.

## What Changes

- The icon sits at two units across and five units down.
- The label rectangle is the tooltip's x less four units, the tooltip's y, six units wide and two units high.
- One begin, one texture bind, two setup calls and two draws are counted.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill tooltip.

## Impact

- `exa_ui::menu`, from `draw_skill_tip`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 222-byte tooltip method's table row, two draws and four unit products. No new corpus files.
- Out of scope: the table's contents.

## Phase

Phase 7, the interface. The tooltip layout.
