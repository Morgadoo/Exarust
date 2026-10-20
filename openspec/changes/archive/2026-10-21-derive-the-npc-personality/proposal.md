# Proposal

## Why

The NPC brain read its traits only for the hostility terms; the
personality the original's AI constructor derives from the record (skill,
confidence, restraint, courage, eagerness, blend, jitter, arousal) and the
attach step's choice of contact sense and statue mode were missing.

## What Changes

- Derive the personality from the combat skill, the second and third
  traits and the feral level, with the jitter from the engine's generator.
- Choose the contact sense (ally for race class `0x12xx`), the dead and
  statue cases and the effect level when the brain is attached.
- Level characters' brains carry their personality.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-perception`: the personality and the attach step.

## Impact

- `exa_ai::personality`, `exa_ai::brain`, `exarust` (placed characters).
- Evidence: the original's AI constructor and attach method (REA). No
  format field changes, so the corpus step is a skip.
- Out of scope: the readers of the courage, eagerness, blend and jitter
  (morale and the combat state), and the effect the attach sets.

## Phase

Phase 6, the NPC brain.
