# Spec Delta

## ADDED Requirements

### Requirement: Sliding door
A sliding creak SHALL stop at energy 0.015 or below. Otherwise the volume SHALL be 0.2 times the square root of the energy and the pitch SHALL be 0.2 times the energy plus 0.7, at most 1. A motorised open SHALL drop the fraction by 0.01 a tick with smoothing 0.1 and stop below 0.01. A close SHALL add 0.075 with smoothing 0.05 and latch past 0.99. Shut and not held, the fraction SHALL stay 1 with smoothing 0.5. Held below 0.995, a pull of 4 SHALL unlatch on the seventh tick from 20.

#### Scenario: Creak, crawl, close, and the seventh pull
- **WHEN** the stored energy is 0.25, a motorised open at fraction 0.015 steps once, a close at fraction 0.95 steps once, a shut door is not held, and a shut door at count 20 and fraction 0.9 is held for seven ticks
- **THEN** the creak volume is 0.1 and the pitch is 0.75, the open fraction is 0 with smoothing 0.1, the close fraction is 1 and it is latched, the shut smoothing is 0.5, and the seventh pull unlatches

### Requirement: Sliding open
A plain open SHALL set the fraction to 0.6 for 80 ticks. A frozen door SHALL keep its stored fraction and stay passable. A hand point past 60 cm SHALL be pulled back to 60.

#### Scenario: A jump, a parked door, and the hand
- **WHEN** a plain open begins, a frozen door is parked at 0.4, and a hand offset is 100 cm along X
- **THEN** the fraction is 0.6 for 80 ticks, the parked fraction stays 0.4 and the door is passable, and the offset is 60 cm
