# Proposal

## Why

The motile body (TMotileDynamics, 53 %) still lacked four of the rules its
task list names: the plain start capture's half-lean of the lumbar and
thorax, the may-act gate's use of the trailing-foot request, the struck
item's blow sound, and the placement's resets before its first sample.

## What Changes

- `exa_physics::body::locomotion`: `half_lean` (a midline bone's captured
  turn keeps half of the twist), `Locomotor::twist_before` (the twist a
  start made earlier in the tick captures against), `Locomotor::placed`
  (the placement's twist, reach and yaw reset).
- `exa_physics::body`: `Ragdoll::clear_leg_corrections` (the placement's
  IK correction reset).
- `exa_game`: the sampler's clip clock half-leans a plain capture's
  lumbar and thorax (`limp::Lean`) — live.
- `exa_ai`: the duellist feeds the trailing-foot request into the step
  gate's blocking word — live.
- `exa_combat::sounds`: `struck_item_sound` (the record's class set below
  its split along the item, else its own sound; the 200 − round(80·s) ms
  clock; pitch × 1.2 in double) — tested.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the start capture's half-lean, the step gate's plant
  request, the struck item's sound, the placement resets.

## Impact

- `exa_physics`, `exa_game`, `exa_ai`, `exa_combat` and their READMEs.
- Evidence: REA/Ghidra and the full disassembly of the start capture, the
  may-act gate, the body tick's plant-request lapse, the damage routine's
  struck-item voice and the placement routine, read as instructions.
- Out of scope: calling the placement resets (a spawned body already starts
  in that state; the round-23 spawn change that moved the controller was
  reverted), feeding `struck_item_sound` from item records and playing it,
  the capture's planted-foot flag, the region-trigger routine the plant
  lapse calls (character AI, player only), the gate's tone and support
  inputs (still full), the rsqrtss estimates.

## Phase

Phase 3, bodies: skeleton, physics, control (TMotileDynamics kernel lane).
