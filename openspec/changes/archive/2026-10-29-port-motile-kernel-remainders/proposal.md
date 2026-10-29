## Why

The TMotileDynamics routine map (`docs/re/motiledyn-map.md`) lists the largest unported pieces of the character body class: the body tick kernel's alignment tail, the damage routine's tail, the feet routine's modes and the character init's remainder. This change ports what the assembly states plainly, wires only what is a clear difference or golem-only, and keeps the rest as tested rules.

Phase 4 (combat and items) and Phase 3 (physical bodies) in docs/PLAN.md.

## What Changes

- A body that does not locomote has no locomotion drive (the body tick clears it every tick before its locomotion call). Live: a recovery step after a walk is no longer pushed by the walk's last drive.
- A golem's body links take the smoothing 0.995 at spawn, as the character init does. Live for golems only.
- The damage routine's pain sounds (wound sound and groan or grunt) and the player's feedback (camera jolt, pad rumble), as tested rules.
- The feet routine's flat-floor and hovering modes and the init's leg order, as tested rules.
- The routine map's statuses, live column and totals; the crate READMEs.

## Evidence

The motiledyn pack's assembly and `exe.asm` (the body tick's alignment tail, the damage routine after its damage call, the feet routine's two early branches, the character init's golem loop and leg order) and the constants read from the PE.

## Out of scope

The down body's ground probe, the scripted damage hook and the damage callbacks, playing voice samples, the camera jolt and pad rumble in the game, the ghouls' head-part set-up, the pose-only sampler's remainder.
