# Proposal

## Why

The dead entry rewrites parts and knocks the body out. It still skips the race setup and the three calls that follow: filling empty slots, flattening the facing, and scaling the step reach by 80.

## What Changes

- When both pair words are 0, the race class and the body kind choose the two hand poses, the bare mark, and clear the class word.
- Empty hand slots are filled, and an empty pair fills the two extras.
- The facing is the offset with its height removed, then normalised.
- The step reach is the base times 80, and times (1 − 0.3 × feral) when feral is not negative.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the race hands, the slots, the facing and the step reach on the dead entry.

## Impact

- `exa_ai::idle::knock_out_dead`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the dead-state entry's race setup and the three calls after the part loops. No new corpus files.
- Out of scope: the virtual call, the weapon pass inside the step, and a blow calling the knock-out.

## Phase

Phase 6, AI. The rest of the dead entry.
