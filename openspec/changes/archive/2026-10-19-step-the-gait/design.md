# Design

## Context

The capsule still stands in for the carry. Sprint overlay and clock already use the feral sprint row. Feet plant only while standing. Blows above half impact add 10 cm/tick per unit of impact.

## Goals / Non-Goals

**Goals:**

- Sprint speed follows the sprint thrust relative to the jog thrust.
- Backing plays Walk backwards and the clock wraps below zero.
- Walking feet follow the phase plant and lift.
- A heavy blow starts a knockback step the walk does not clear.

**Non-Goals:**

- Replacing the capsule with the carry force.
- The 0.4 sprint run cap, stride modulation, Force Strikes above 0.

## Decisions

1. Capsule speed stays measured for walk (1.4 m/s) and jog (3.6 m/s). Sprint scales the jog end by sprint_thrust / 350. The feral blend is the same column mix as the sprint gait.
2. `exploration_gait` returns Walk for any move. `Pace.backing` selects the negated backpedal rate (28/83). Action 2, if played, uses that same negative rate.
3. Phase is `rem_euclid(progress + 1.2 − 0.5×leg, 1)`. Plant and lift run before footwork, and walking skips footwork.
4. `knockback_offset` returns metres. The hit path stores it on the shove. The solver starts the step at the chest plus that offset. `consider` keeps a knockback step while walking, and ends it under balance 0.2 or while getting up. The thorax lead uses kind 1.

## Risks / Trade-offs

- Jog and sprint equilibrium speeds from the carry are not measured. The thrust ratio is the documented column, applied to the existing jog speed. Label: the capsule is still a stand-in.
- One foot is down during part of a walk cycle. Tests that assumed every walking foot was clear now use a progress outside both plant windows when they need both feet up.

## Migration Plan

No data migration. Archive updates the main specs.

## Open Questions

None. The constants are the ones already written down from the original's gait table, sampler and damage routine.
