# Proposal

## Why

The skill screen's draw opens with a fade: state byte 4 shows nothing, a set bit 0 fades toward the deadline with a quadratic curve, and a passed deadline clears the bit. Exarust has the screen's inputs and searches but not the fade.

## What Changes

- State byte 4 returns alpha and sound 0.
- Bit 0 clear returns alpha and sound 1.
- Bit 0 set with the deadline at or below the clock clears the bit and returns 1.
- Otherwise the fade uses `t = (deadline − clock) × 0.002`, `x = (1 − t)(1 + t)`, alpha the smaller of x and 1, and sound `x(2 − x)`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's fade.

## Impact

- `exa_ui::skill`, from `skill_screen_fade`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: no coverage change; this is part of the still-open 2,955-byte draw.
- Evidence: the 2,955-byte `0x100226520` opening: the state byte `0x158` against 4, bit 0, the deadline `0x160` against the clock `0x1004997d0`, the double `0.002` at `0x10033ad30`, `1.0` at `0x10033ad28`, `2.0` at `0x10033ad38`, the clamped alpha and the `0x10002a350` sound call at code `0x23`. No new corpus files.
- Out of scope: the rest of the draw.

## Phase

Phase 4, UI. The skill screen's fade.
