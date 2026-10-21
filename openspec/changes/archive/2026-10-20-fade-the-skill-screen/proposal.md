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
- Evidence: the 2,955-byte skill-screen tick's opening: the state byte against 4, bit 0, the deadline against the game clock, the doubles `0.002`, `1.0` and `2.0`, the clamped alpha and the sound call with code `0x23`. No new corpus files.
- Out of scope: the rest of the draw.

## Phase

Phase 4, UI. The skill screen's fade.
