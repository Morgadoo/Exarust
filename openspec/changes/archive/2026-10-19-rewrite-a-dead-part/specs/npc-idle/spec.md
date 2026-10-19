# Spec Delta

## ADDED Requirements

### Requirement: A dead entry rewrites parts before the knock-out
A dead entry SHALL set a held part's mark to 0x2001 from 1, its gate to 6 from 15, its span bits to 0x40000000, and its identity turn to 0, 0, 0, 1. An unheld part SHALL keep mark 3 and span 9 and still take gate 6. A negative basis SHALL take turn 0, 0, 1, 0. A link of count 1 SHALL become kind 4 and bits 0x30, and count 0 SHALL stay. The body SHALL then be down at clock 50. A walking body SHALL keep its parts and stay walking.

#### Scenario: Held part, plain part, and a walking body
- **WHEN** a dead body with a held part, an unheld part, a negative basis, a counted link and an empty link is entered at clock 50, and a walking body is entered with those same parts
- **THEN** the held mark is 0x2001, the gate is 6, the span bits are 0x40000000, the identity turn is 0, 0, 0, 1, the unheld mark stays 3 and the span stays 9, the negative turn is 0, 0, 1, 0, the counted link is kind 4 and bits 0x30, the empty link stays, the dead body is down at 50, and the walking body keeps its parts and stays walking
