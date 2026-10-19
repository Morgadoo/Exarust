# Proposal

## Why

The line class has a hit test for the rail and the scrollbar that Exarust does not. It maps a point to one of six codes from the place's flags and limits.

## What Changes

- The span is four units, or six when the wide flag is set.
- A trim flag of 0x10 and a point inside the span run the thumb test: code -4 when the second control bit is set and the point is at or past the right limit minus the span, else code -3 when the first bit is set.
- Otherwise, control bit 0x200 and a point past the left limit run the strip test: -8 above the bottom limit, -10 inside the bottom band, -9 near the top limit.
- Any other point returns 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the line class's rail hit test.

## Impact

- `exa_ui::skin`, from `hit_line`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class hit method and its six return codes. The meaning of each code is not established, so the function returns the code unchanged. No new corpus files.
- Out of scope: what the caller does with a code.

## Phase

Phase 7, the interface. The code, not its use.
