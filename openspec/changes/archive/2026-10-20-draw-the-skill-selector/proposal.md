# Proposal

## Why

The skill selector draws either a title on page 2 or the selected skill's icon and description below it. Exarust does not model this draw.

## What Changes

- Page 2 centres a text line at 12 units across, the place width less two units and 20 units high.
- Any other page draws the row indexed by the low 16 bits less one when it is in 0 through 7: the measured icon centred at 16 units down, then a label at 18 units across, the place width less two units and the height less 20 units.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill selector's draw.

## Impact

- `exa_ui::menu`, from `draw_skill_selector`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 424-byte selector draw's two branches and its unit products. No new corpus files.
- Out of scope: the row table and the text measurement.

## Phase

Phase 7, the interface. The selector's layout.
