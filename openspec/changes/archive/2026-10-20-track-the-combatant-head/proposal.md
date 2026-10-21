# Proposal

## Why

The combatant's per-tick head tracking (1,330 bytes) aims the head at the body's point of interest through two relaxed angles, the engine's planar angle, a 1.8° step and a ±45° body clamp. Exarust has the combatant's reset, measures and positioning but not the head tracking.

## What Changes

- The look direction is the look-at minus the eye; under 50 cm the previous direction is reused, and in the 50–100 cm band the vertical component becomes `(10000 − d²)·1e-4` before normalizing.
- Yaw chases the approximate xz atan2 by at most π/100 a tick and is held within ±π/4 of the body's facing.
- Pitch chases the floored negative asin by at most π/100 a tick, with a −0.6 floor.
- The result is the unit direction, the point 250 cm along it and the flat unit direction.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the combatant's head tracking.

## Impact

- `exa_combat::aim`, from `HeadTrack`, `HeadLook`, `track_head`, `head_atan2_xz`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TCombatant` becomes **30%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the head tracking: the look-at virtual, the eye from the body's frame × (−20, 10, 0), the 2,500/10,000 squared radii and the 1e-4 band slope, the π/100 step and ±π/4 clamps, the −0.6 pitch floor, the `(cos p sin y, −sin p, cos p cos y)` direction and the 250 cm point, with the planar-angle helper and the arc-sine polynomial. No new corpus files.
- Out of scope: the caller's eye frame and the point-of-interest selection.

## Phase

Phase 5, combat bodies. The combatant's head tracking.
