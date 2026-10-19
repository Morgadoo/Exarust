# Proposal

## Why

The dead-state entry knocks a body out. Before that it rewrites each part: a mark, a span, a turn from the basis, and two cleared gate bits.

## What Changes

- A held part gains mark bit 0x2000, keeps its span times 1, and takes a turn from its basis.
- Every part clears gate bits 0 and 3.
- A link whose count is above 0 becomes kind 4 and bits 0x30.
- The marker clears, a product and a halved step are stored, and posed and the behaviour mark become 1. Then the body is knocked out.
- A body that is not dead is left alone. A blow does not call this.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the part rewrites before the dead knock-out.

## Impact

- `exa_ai::idle::knock_out_dead`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the dead-state entry's part loops. No new corpus files.
- Out of scope: the race setup, the virtual call, the three calls after the loops, and a blow calling the knock-out.

## Phase

Phase 6, AI. The dead-entry part rewrite.
