# Proposal

## Why

The line class draws a small square of four units after the span square. The next method draws it from a place and a flag word, with a second sample pass when bit 0x40 is set.

## What Changes

- At unit 5, place 10, 20 draws (10, 40), (10, 20), (30, 20), (30, 40).
- The sample is 0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8.
- Flag 0x40 binds the second sample, 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8, and draws the same corners again.
- Flag 0x10 selects the first ink. Mode 0x10000 selects the second ink with its alpha forced to 255. Any other flag word selects the third ink.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the four-unit chip on the line class.

## Impact

- `exa_ui::skin`, from `draw_line_chip`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class method that draws four units, its two sample arrays, and the three ink reads. No new corpus files.
- Out of scope: the methods around it and the text bind those materials select.

## Phase

Phase 7, the interface. The chip, not the neighbouring methods.
