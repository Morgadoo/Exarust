# Proposal

## Why

The line class has a shade draw that no Exarust code models: a six-quad bar across the place, then a solid inner rectangle inset by the unit.

## What Changes

- Unit 5 at place 10, 20 of size 100 by 80 with a side inset of 15 draws six outer quads from y 13 to 23 and from y 97 to 107, spanning x 10 to 110 around inset edges 25 and 95.
- The binds are cap, bar, cap, cap, bar, cap.
- It then draws an inner rectangle from 15, 25 to 105, 95.
- Three mode calls (2, 1, 0x10), one preparation, two begins, two ends, twenty-eight vertices and twenty-eight colour writes are counted.
- State band 0x10000 selects the special ink with alpha 255; any other state uses the normal ink. The inner ink always takes alpha 255.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the line-class shade.

## Impact

- `exa_ui::skin`, from `draw_line_shade`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the shade method, the bar method it calls, the rectangle helper and the two ink fields. No new corpus files.
- Out of scope: the owner's side inset and the methods around it.

## Phase

Phase 7, the interface. The shade, not its owner.
