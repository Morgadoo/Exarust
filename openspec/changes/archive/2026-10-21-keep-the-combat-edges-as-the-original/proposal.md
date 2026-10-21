# Proposal

## Why

A review of the NPC combat state against the original's instructions
found a few edges the port did not keep: the direction left unwritten
within 1 cm of the estimate, the keep-off clamp's unordered compares, the
dead foe that ends combat even with a focus, and a second copy of the
keep-off clamp in the AI manager that tested the vector as computed
instead of as the original reads it.

## What Changes

- The estimate keeps the last direction within 1 cm (or with a distance
  that is not a number).
- The keep-off clamp takes its outside branch for a distance that is not
  a number and cuts the step when its test cosine is not a number.
- A foe clears the focus, so a dead foe ends combat.
- The AI manager's approach clamp is the combat state's keep-off (one
  implementation, the original's test vector kept).

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-manager`: the approach clamp's last test reads as the original's.

## Impact

- `exa_ai::combat`, `exa_ai::manager`, the duellist's can-step gate
  comment.
- Evidence: REA on the original's combat state, its estimate and leave
  helpers and the keep-off clamp (instructions read for the compares).
  No format field changes, so the corpus step is a skip.
- Out of scope: the rest of the original's can-act test (grip, awareness,
  blocking state), which the duellist does not have.

## Phase

Phase 6, the NPC combat AI.
