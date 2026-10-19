# Proposal

## Why

Inside the view the hit was still supplied, and a new pane only counted a build.

## What Changes

- A scene with sectors skips a sector that shares no camera bit. A body is visited when its flags include 0x89, it shares a camera bit, and it lacks bit 0x4000000. A clear probe records the bits of 0.1. Any other probe records the bits of 5. A positive hit count replaces the hit. An empty scene leaves the supplied hit.
- A new pane doubles the row before the slot, caps it, sizes the inner pane from the unit and the span words, and clamps the place. Bit 0x1000 chooses the wide slot.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the in-view scene walk and the pane size.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the sector and body masks, the probe pads, the doubled row, and the place clamp. No new corpus files.
- Out of scope: the ray distance, and child layout.

## Phase

Phase 7, arena career. This walk and this size, not the ray distance.
