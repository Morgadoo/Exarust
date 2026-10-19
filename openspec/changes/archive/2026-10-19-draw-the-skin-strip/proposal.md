# Proposal

## Why

The parent skin draws a four-band textured strip between two x values and a vertical span. Exarust does not model it.

## What Changes

- A begin, four texture binds and eight vertices form four two-vertex bands.
- The x pair is the first and third floats; the y pair is the second and fourth floats.
- The middle y is half the span from the fourth float to the second, plus the fourth.
- The four texture slots are 0x12c, 0x134, 0x138 and 0x130 in that order.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's textured strip.

## Impact

- `exa_ui::skin`, from `draw_skin_strip`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 339-byte parent method's eight vertex calls and four binds. No new corpus files.
- Out of scope: the caller and the texture contents.

## Phase

Phase 7, the interface. The strip geometry, not its texture.
