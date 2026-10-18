# Proposal

## Why

Phase 3 gait still commands a jog-speed capsule while sprinting, plays a Backpedal clip, leaves walking feet untouched, and shoves a heavy blow instead of stepping. `docs/PLAN.md` §7 lists those four next (M16, C16, C19).

## What Changes

- While sprinting, the capsule speed's jog end is the jog speed times the feral sprint row's thrust over the jog thrust.
- Backing plays Walk and runs its clock backwards at the negated backpedal rate. A looping clock below zero wraps by the last key.
- A walking foot plants when its phase is 0.2 to 0.4 and it does not touch, and lifts when it touches and the phase is 0 or at least 0.55.
- Impact over 0.5 starts a knockback step of 2·(20·strikes + 10)·J cm along the swing, plus 20 ticks of chest velocity, capped at the step length. Below that, the old velocity shove stays.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `player-movement`: sprint speed from the sprint row's thrust.
- `motion-playback`: Walk played backwards, and a negative loop wrap.
- `physical-body`: gait plants, and a knockback step that walking does not clear.
- `combat`: a heavy blow's step replaces the flat impact shove.

## Impact

- `exa_game` steer and clip clock, `exa_anim` clock wrap, `exa_physics` feet and steps, `exa_combat` hit reaction.
- `docs/PLAN.md`, `docs/PLAYER.md`, `docs/formats/RSQ.md`, `docs/formats/OTHER.md`.
- Evidence: the movement notes' gait rows and locomotion drive, the feet notes' sampler plants, and the damage notes' knockback step, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: the tighter run cap, stride modulation, removing the capsule, inventory, blocking and parrying, and Force Strikes above 0.
