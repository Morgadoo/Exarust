# Proposal

## Why

The line class still has no bevel box after the chip. The next method draws one from a place and a state word: a gradient fill, a six-line border and a four-line outline, each arm with its own colours.

## What Changes

- Place 10, 20 of size 100 by 80 draws a fill from 12, 22 to 108, 98 as two gradient quads with a hard seam at 60.
- Its border runs from 11.5, 21.5 to 108.5, 98.5 as six lines. The left seam sits at 60.5 and the right seam at 60.
- Its outline runs from 10.5, 20.5 to 109.5, 99.5 as four lines.
- Bit 0x40 selects the raised arm, bit 0x10 the pressed arm, and neither selects the idle arm.
- Each arm supplies two fill colours, two border colours and one outline colour. Fill stops are 0.6 and 0.4, border stops 0.65 and 0.35.
- The shipped inks make the raised and idle arms fully transparent and the pressed arm black with alpha 0xE0. The insets of one and two pixels are literal and do not follow the unit.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bevel box on the line class.

## Impact

- `exa_ui::skin`, from `draw_bevel`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class method at the slot after the square, its three colour pairs per arm, its two fills, six border lines and four outline lines, and the shipped scheme. No new corpus files.
- Out of scope: the colour ramps that fill the arm inks and the methods around it.

## Phase

Phase 7, the interface. The bevel box, not the ramps.
