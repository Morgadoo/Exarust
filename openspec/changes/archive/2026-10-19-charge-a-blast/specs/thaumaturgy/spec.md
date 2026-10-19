# Spec Delta

## ADDED Requirements

### Requirement: Blast charge
A tick of N milliseconds SHALL set the rate to N thousandths, then five sixths of that. While charging, the amount SHALL rise by the rate, and passing 1 SHALL clear it, set the state to 1 and count as a release. An amount of exactly 1 SHALL keep charging. While fizzling, the amount SHALL fall by twice the rate, and passing below 0 SHALL become 0 with state -2. After release the amount SHALL rise by three times the rate.

#### Scenario: A full charge, a fizzle and a release
- **WHEN** the rate of 12 ms and of 1000 ms is read, 0.5 is charged by 0.5, 0.6 is charged by 0.5, 0.5 is fizzled by 0.1, 0.3 is fizzled by 0.2, and 0.2 is stepped after release by 0.1
- **THEN** the rates are 0.01 and five sixths, the full amount stays at 1 and charging, the larger charge releases at 0 with state 1, the mild fizzle is 0.3, the deep fizzle is 0 with state -2, and the released amount is 0.5

### Requirement: Blast chunks
A chunk SHALL grow by the tick times 0.7 of its speed plus 3, times 0.0006. Two chunks closer than twice the first radius SHALL move apart by half the overlap. A pair already that far apart, or on the same point, SHALL stay.

#### Scenario: A still chunk and a close pair
- **WHEN** a chunk of radius 0 and speed 0 grows for one tick, chunks at 0 and 1 with radius 1 are separated, a pair 2 apart is separated, and a pair on the same point is separated
- **THEN** the still chunk is 0.0018, the close pair is at -0.5 and 1.5, and the other two pairs stay
