## Why

PLAN Phase 5 (the physical body). The TMotileDynamics gap lane's next
reachable remainder: the pose-only sampler (routine #76 of
`docs/re/motiledyn-map.md`) places each part on its own route, and
Exarust's placement on the ground posed every part on the plain clip chain.

## Evidence

REA (Exanima.exe), the pose-only sampler's part loop (pseudo-code, checked
in the assembly): a thigh's local turn is multiplied by the femur
posture's turn before the chain; a part with the twist flag takes its chain
times the twist, the one under the root half way; the head clears that flag
and is measured from its parent's untwisted chain; the part with the support
pins takes the thorax hunch when the feral level is over 0.1. The helper
that writes each node's local is the relative transform (Aᵀ·(B − t)), so the
twist does not reach the children. No leg IK, look or foot pitch runs there.

## What Changes

- exa_game: `limp::route_placement` and `PlacementRouting` (femur posture,
  twist, feral level); `place_on_ground` routes the posed parts (the
  player's third tick, the AI's re-entry). The first spawn passes no routing
  (every input is the identity there). `stages::sampler`'s femur posture,
  thorax hunch and spine are shared with it.
- docs/re/motiledyn-map.md: #76.

## Impact

The player's ground placement in the arena takes the combat femur posture
and its small twist, so the arena duel diverges from main from tick 3;
exanima01's first re-entered feral body takes the hunch, and its trace
diverges at tick 347 in that body alone.
