# Spec Delta

## Purpose
One playable effect for each thaumaturgy domain, paid for with focus.

## ADDED Requirements

### Requirement: Four effects
Displacement SHALL place the body at the chosen point and spend 0.20 focus. Energy SHALL lower yellow health by the arc strength, not again until 150 ms have passed, and spend 0.10 focus. Force SHALL knock 60 along the horizontal aim and spend 0.12 focus. Mind SHALL lower each concern by 30, aggression by 10 and fear by 50, floored at zero, and spend 0.3 times 0.14 focus. A cast without the focus SHALL change nothing.

#### Scenario: Each domain
- **WHEN** each domain is cast with enough focus, and a cast is tried below its cost
- **THEN** the body moves, yellow health falls, the knock is 60, calm lowers the mind, and the short cast leaves focus and the target unchanged
