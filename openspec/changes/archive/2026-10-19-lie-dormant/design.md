# Design

## Context

Roam, patrol, sentry, a wander point and follow are in. Sleep is the idle that lies as a corpse until it sees a target.

## Goals / Non-Goals

**Goals:**

- Set the dormant condition and the dormant flag, keeping any other flags.
- Zero the tone, so the tone target is 0 too.
- Mark a statue and set gait 2.
- Install a tick that writes nothing.

**Non-Goals:**

- Waking when a target is seen.
- The statue countdown of the body solver.

## Decisions

1. The condition is replaced with dormant. The flag is OR-ed, so a flag already set stays.
2. Tone 0 is what the body's tone target multiplies, so the target is 0 without a second store.
3. Gait 2 is the value the pelvis rest treats as the taller offset. The statue mark is the byte the solver counts down for.
4. The tick is the empty one sleep installs. Calling it leaves the sleeper and the tone as they were.

## Risks / Trade-offs

- Sleep is not yet called from a placed character. The solver does not yet count a statue down.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The sleep step was read before this change.
