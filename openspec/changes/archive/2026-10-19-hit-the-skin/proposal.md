# Proposal

## Why

The parent skin has its own hit test: two edge flags return combined edge codes, then a threshold band probes three codes, and the right and bottom edges probe two each before falling back. Exarust models the line class's hit test but not this parent one.

## What Changes

- Flags 0x4000 and 0x8000 test the left/right and top/bottom halves and return 0x80000011/0x80000012 and 0x80000014/0x80000018.
- If no edge fired, flags bit 0 and a y above the threshold probe codes -4, -5 and -10 under flag bits 1, 2 and 3, then fall back to -3.
- Flag 0x200 with x at or past the stored limit probes -9 and -10, then falls back to -8.
- Flag 0x100 with y at or past the stored limit probes -14 and -13, then falls back to -12.
- A probe succeeds when the code's rectangle contains the point inclusively.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's hit test.

## Impact

- `exa_ui::skin`, from `hit_skin`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 807-byte parent method and its 52-byte probe helper. No new corpus files.
- Out of scope: the virtual that supplies each code's rectangle.

## Phase

Phase 7, the interface. The hit test, not the caller.
