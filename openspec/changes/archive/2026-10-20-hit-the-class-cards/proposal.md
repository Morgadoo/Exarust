# Proposal

## Why

The class-select screen maps a point to one of up to four cards laid out from the clock scale and the available-class mask. Exarust does not model that hit test.

## What Changes

- The band top is half of the clock less 1024 scale steps, plus 140 steps; the height is 512 steps.
- The card count is one plus the mask's first three bits.
- Cards are laid left to right, 288 steps apart, each 256 steps wide, centred on the screen's half width.
- A point inside a band and card, exclusive on both edges, names that card; one outside names none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the class-card hit test.

## Impact

- `exa_ui::menu`, from `hit_class_cards`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 465-byte hit test and its recovered 512/256/288/140 scale constants. No new corpus files.
- Out of scope: the class-card draw.

## Phase

Phase 7, the interface. The card hit test.
