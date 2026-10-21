# Proposal

## Why

The motile body's tick kernel still lacked its swing-side body rules (the
aim nudges, the attack arms' stiffness, the parry trigger, tangle and clinch
clears, the held weapon's handling pushes, the blow stagger and recoil,
burning), the walk driven by the carry, the trailing-foot plant and the
femur posture. A first pass wrote them from decompiler pseudo-code and
several diverged from the executable's instructions (float order, hidden
sign compares, double arithmetic, invented fallbacks).

## What Changes

- `exa_combat`: swing curves, weapon gate, sound, fatigue and attack-arm
  stiffness (`swing`); the strike body's nudges (`strike_body`); the parry
  trigger (`parry`); the held weapon's handling pushes (`weapon`); burning
  (`burn`); the ready-state effects (`effects`).
- `exa_physics::body`: step phase and step end, full tone, trailing-foot
  plant, point push weighted by the point's own travel per tick, femur
  posture, stagger and recoil, the carry walk, head tilt and exertion.
- `exa_audio`: the footstep set choice (no invented wet fallback) and the
  voice choice by the feral level.
- `exa_game`: the sampler poses the thighs in the combat conditions; the
  movement stage walks muscled bodies by their carry and feeds the head
  tilt, exertion, look target and start notice.
- Corrections found against the instructions: the step phase in double with
  the engine's epsilon, the step end's dot order, the tone reset's stance,
  the push fraction's reference vector, the rearmost-foot search start, the
  smooth-step grouping, edge/overhead/fist/two-hand/clinch details, the
  weapon gate's squared distance, the attack arms' stances.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the combat-side body rules and the foot plant.

## Impact

- `exa_combat`, `exa_physics` (`body*`), `exa_audio` (`voices`),
  `exa_game` (`stages::movement`, `stages::sampler`), their READMEs and
  `docs/formats/RSQ.md`.
- Evidence: REA/Ghidra disassembly of the motile body's tick, balance and
  swing routines and their helpers, read as instructions rather than
  pseudo-code; constants from the executable's data. Tests use worked
  numbers checked against the instruction semantics.
- Out of scope: running the swing, parry, weapon, stagger, recoil and burn
  rules from the game's combat stage (they are library code, not called yet),
  the start callback, the grip link bodies, the full three-bone IK, the
  stand-still timers of the trailing foot, the reciprocal-square-root
  estimates, the voices' wiring.

## Phase

Phase 3, bodies: skeleton, physics, control, with Phase 4's swing rules
(TMotileDynamics kernel lane).
