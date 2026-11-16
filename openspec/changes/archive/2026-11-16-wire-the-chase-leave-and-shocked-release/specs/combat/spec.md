## ADDED Requirements

### Requirement: Cast start clinch clear
A cast motion started while the body parries SHALL first end the clinch: the parry is off, keeps no clip and counters no motion, the hands lose the arm bit and every held item's parry link is disabled. A body that does not parry SHALL keep its parry state.

#### Scenario: A cast while parrying
- **WHEN** a parrying body starts a cast motion
- **THEN** its parry is off with no clip and no countered motion

### Requirement: Shocked release in combat
A released race shocked while attacking SHALL have its swing reset: stall 1, strength 0, the stalling and cleaving bits set, ending 600 ms after the shock.

#### Scenario: A released attacker is shocked
- **WHEN** a released race in condition 2 takes a shock
- **THEN** it is shocked and its swing stalls with no strength until 600 ms on

### Requirement: Dying released race not shocked
A released race that is collapsing or dead SHALL take no shock: no hit sound, no draw, and its next shock time unchanged.

#### Scenario: A collapsing released race
- **WHEN** a collapsing released race is shocked
- **THEN** its condition and its shock clock stay as they were
