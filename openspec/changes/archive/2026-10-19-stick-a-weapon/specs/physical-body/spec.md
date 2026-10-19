# Spec Delta

## ADDED Requirements

### Requirement: Weapon stick
An armed bite above 0.1 SHALL stick. Depth SHALL cap at 1. Above a promoted 0.4 the flags SHALL be 0x211000, and float 0.4 SHALL too. A lower depth SHALL use 0x11000. Strength SHALL be 12 times the depth. Smoothing SHALL add 0.5. The count SHALL be 3 times the truncated product of 83, the unit and the depth, and the span twice that product. Depth 3 SHALL match depth 1. The weapon layer SHALL be 0xC0. The part, its parent and each child SHALL gain bit 0x40.

#### Scenario: A tenth stays free and a deep bite caps
- **WHEN** an unarmed depth of 1 is stuck, a depth of 0.1 is stuck, a depth of 0.25 with smoothing 0.1 and unit 1 is stuck, a float depth of 0.4 with smoothing 0 and unit 1 is stuck, a depth of 1 and a depth of 3 are stuck, a depth of 0.11 with unit 2 is stuck, and a part layer of 3 with a parent of 0 and children 1 and 0x40 is marked
- **THEN** the unarmed bite and the tenth stick nothing, 0.25 uses flags 0x11000, strength 3, smoothing 0.6, count 60 and span 41.5, 0.4 uses flags 0x211000, count 99 and span bits 0x4284cccd, depth 1 uses count 249 and span 166, depth 3 matches that count, span and strength with smoothing 1, unit 2 uses count 54, the weapon layer is 0xC0, and the marked layers are 0x40, 0x43, 0x41 and 0x40
