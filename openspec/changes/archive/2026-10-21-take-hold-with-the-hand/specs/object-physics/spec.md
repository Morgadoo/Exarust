# Spec Delta

## ADDED Requirements

### Requirement: The hand's grab
A limp body SHALL let a grab pull a point of one part toward a world anchor, right after that part's own links, in each pass up to the grab's last pass (counted from 0 each tick) or in every pass. The pull SHALL move each particle by its weight, taken by the part alone.

#### Scenario: A crate dragged across a floor
- **WHEN** a resting 40 cm crate of mobility 0.5 is grabbed by its middle at strength 1 for the first two passes and its anchor is put 1 m away across the floor for 300 ticks
- **THEN** it never moves more than 10 cm a tick, the grabbed point stops within 5 cm of the anchor with the crate still on the floor, and let go it settles and sleeps

### Requirement: The grab's pull
The gap SHALL be taken across the ground only; under 1.19209e-7 cm² it SHALL do nothing. Otherwise the pull SHALL be twice the strength times the gap, the gap counted at most 3 times the root of the part's mobility in cm and never more than 5 cm. The share of the point's motion since the tick began that heads toward the anchor SHALL be taken off when it is over 1.19209e-7 cm.

#### Scenario: The pull and its cap
- **WHEN** a part of mobility 1 is pulled at strength 1 from 2 cm away (9 cm below), then from 10 cm away, a part of mobility 4 at strength 0.5 from 10 cm away, and a gap of 3e-4 cm
- **THEN** the pulls are 4 cm, 6 cm and 5 cm across the ground, and the last pulls nothing

#### Scenario: Approaching motion
- **WHEN** the point already moved 1.5 cm toward an anchor 10 cm away on a part of mobility 1
- **THEN** the pull is 4.5 cm; moving 1.5 cm away it stays 6 cm
