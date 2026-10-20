# Proposal

## Why

The body already reads a crouch amount (the pelvis lowers toward 60 cm,
the chest shifts back, the support pins and the footwork's stance scores
narrow), but nothing ever set it: the X binding did nothing, and the
locomotion clock, the footwork's step length and the attack pick ignored
crouching.

## What Changes

- Holding the Crouch binding (X) while the body is not moving, or while it
  is down, sets the crouch amount to 1. Every tick the amount falls by 0.02
  while it is above 0, so a released crouch eases out over 50 ticks.
- The body's bearing carries the player's crouch to the physics.
- Above a crouch of 0.1 the locomotion clip clock runs × (1 − 0.1·crouch).
- Above a crouch of 0.5 the footwork's step length is × 0.7.
- A crouched attacker (crouch above 0) always picks the first side attack.

## Capabilities

### Modified Capabilities

- `player-movement`: crouch.

## Impact

- `exa_game::player`: `PlayerBody::crouch`, the crouch tick in the steering,
  the crouched attack pick.
- `exa_game::limp`: the bearing's crouch and the clock factor.
- `exa_physics::body::steps`: the crouched stride factor on the footwork's
  step length.
- `docs/PLAYER.md` (Crouch).
- Evidence (read from `Exanima.exe`): the player-control routine sets the
  crouch word to 1.0 when the crouch bit is held and the body is not
  locomoting or is flagged down, then sets the scaled step length to the
  step length × 0.7 above a crouch of 0.5 (× 1.5 at gait index 2 or more);
  the body tick subtracts 0.02 (double) while the crouch is above 0; the
  locomotion routine multiplies the gait record's clip rate by
  (1 − 0.1·crouch) above 0.1; the side-1 starter forces index 0 while
  crouched. Notes: `context/re/master-combat.md` §1.1,
  `context/re/body-feet.md`, `context/re/master-animation.md` §4.2.
- Out of scope: the 1.5 long-step factor, an AI crouch, crouch-specific
  clips (the original has none; the crouch is the physics' lowered pelvis),
  and the crouched twist offset of the look.

## Phase

Phase 3, player behaviour: crouch.
