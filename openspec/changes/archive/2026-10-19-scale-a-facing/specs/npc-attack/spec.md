# Spec Delta

## ADDED Requirements

### Requirement: A facing scales the place
A fighter who is not attacking, with the candidate beside it, SHALL score approach bits 0x3f400000 at 25 cm. Straight ahead SHALL keep bits 0x3ec00000. A press with the opponent facing aside SHALL score strike bits 0x40300000 and approach bits 0x40840000.

#### Scenario: Beside, ahead and a press
- **WHEN** a fight at 25 cm is scored while not attacking with the candidate beside it, then straight ahead, then pressing while the opponent faces aside
- **THEN** the approach bits are 0x3f400000 and 0x3ec00000, and the press scores strike bits 0x40300000 and approach bits 0x40840000

### Requirement: An idle stamp recovers the arc
An armed action outside a swing SHALL publish kind 4 when the idle stamp is 5 and the clock is 4, with both edges on the heading. Stamp 4 against clock 4, and stamp 0 against the maximum clock, SHALL publish kind 0.

#### Scenario: Stamp above the clock, and not above it
- **WHEN** an armed action outside a swing is published with stamp 5 and clock 4, with stamp 4 and clock 4, and with stamp 0 and the maximum clock
- **THEN** the first publish is kind 4 with both edges on the heading, and the other two are kind 0
