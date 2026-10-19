# Spec Delta

## ADDED Requirements

### Requirement: Lever step
A lever's creak SHALL smooth the arm velocity by half and stop at a squared speed of 0.015 or below. Otherwise the volume SHALL be 0.2 times the square root of that energy and the pitch SHALL be 0.2 times the energy plus 0.7, at most 1. The drive SHALL move 0.02 rad toward the target and SHALL hold once the gap is within 0.02. Held, the target SHALL be the hand. Forced, it SHALL be the off detent. Returning, it SHALL be the stored detent. Otherwise it SHALL be the nearest.

#### Scenario: Creak and a step toward the nearest detent
- **WHEN** a still arm is given a velocity of 1 along X at 0.2 rad from the first of two detents 1 rad apart, a return aims at the second detent from 0.5 rad, and a forced lever sits at 0.4 rad
- **THEN** the creak volume is 0.1 and the pitch is 0.75, the free drive is 0.18, the return drive is 0.52 toward 1, and the forced drive is 0.38 toward 0

### Requirement: Lever signal
Arriving more than 0.1 rad from the last detent SHALL report the rest flag at the first detent and its opposite at the second. A report of 0 SHALL signal 0 unless inverted, and SHALL stop a return. A report of 1 SHALL signal 1, or -1 when forced and inverted, and SHALL arm a timed return. A linked light SHALL fade out on 0 and in on 1. A report of 0 with a light SHALL not signal.

#### Scenario: An arrival signals, and a light fades
- **WHEN** a lever arrives at the first detent from a last detent of 1 with the rest flag clear, another arrives at the second detent with a return time, a held lever is forced and inverted on the first detent with the rest flag set, and a lit lever arrives at the first detent
- **THEN** the first signals 0, the second signals 1 and arms the return, the held lever signals -1, and the lit lever fades out without a signal
