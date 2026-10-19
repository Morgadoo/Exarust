# thaumaturgy Specification

## Purpose
One playable effect for each thaumaturgy domain, paid for with focus.

## Requirements

### Requirement: Four effects
Displacement SHALL place the body at the chosen point and spend 0.20 focus. Energy SHALL lower yellow health by the arc strength, not again until 150 ms have passed, and spend 0.10 focus. Force SHALL knock 60 along the horizontal aim and spend 0.12 focus. Mind SHALL lower each concern by 30, aggression by 10 and fear by 50, floored at zero, and spend 0.3 times 0.14 focus. A cast without the focus SHALL change nothing.

#### Scenario: Each domain
- **WHEN** each domain is cast with enough focus, and a cast is tried below its cost
- **THEN** the body moves, yellow health falls, the knock is 60, calm lowers the mind, and the short cast leaves focus and the target unchanged

### Requirement: Power prices
The power table SHALL record each listed id and its focus cost, including the four effects already cast. A sustained cost SHALL be that cost times the tick in seconds, and half of that when the sustain technique is known. Repeating the same instant power with the repeat technique SHALL halve the cost. Blast and bolt SHALL spend 1.2 times the cost, barrier twice, a mind power 0.3 times, vanish 80 times, and the long door once.

#### Scenario: Barrier tick and a repeated blast
- **WHEN** barrier's 0.07 is priced for a 500 ms tick, then again with the sustain technique, and blast's 0.10 is repeated with the repeat technique
- **THEN** the costs are 0.035, 0.0175 and 0.05, and calm spends 0.3 times its cost while the long door spends its cost once
