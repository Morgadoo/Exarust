# Proposal

## Why

A standing body that is pushed keeps its feet planted and either sways or
falls. The original takes a recovery step: the chest's offset from the feet
picks a target, the footwork's stance score moves one foot, and the episode
ends by its own rules. That is the next Phase 3 gap after the support pins
(`docs/PLAN.md` §7, milestone M3 "standing man").

## What Changes

- A standing body starts a recovery step from the original's reach, energy
  and support, and aims it with the chest's velocity lead.
- The footwork chooses stay, foot 0 or foot 1 by the stance score and the
  step timers, still one foot at a time.
- The episode ends by the original's timeouts and landings, then the
  standing position sits on the best-placed support foot.
- While the step runs and the chest is not out of balance, the thorax pin
  targets lead toward the step before the feet-centre clamp.
- The radius base widens to 65 cm after the body was down and narrows again.

## Capabilities

### New Capabilities

None. Recovery steps belong to the physical body.

### Modified Capabilities

- `physical-body`: adds the recovery-step trigger, target, stance score,
  episode and thorax lead. The existing foot scenarios stay: a spot under
  5 cm still does not step.

## Impact

- `exa_physics` body step: the balance measure, the footwork and the
  support pins. `Bearing` gains whether the Idle clip is playing, so the
  lead uses 0.4 then and 0.7 otherwise.
- `docs/PLAN.md`, `docs/formats/RSQ.md` and the physics crate README.
- Out of scope: leg IK and its stepping lift (C13), gait plants and the
  closing step (M16, C19), combat footwork and knockback steps (C16),
  footstep sounds, and removing the capsule. Knockback's lead factor of
  1 stays out with knockback steps.
