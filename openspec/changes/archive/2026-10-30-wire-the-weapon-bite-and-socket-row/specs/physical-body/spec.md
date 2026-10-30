# Spec Delta

## MODIFIED Requirements

### Requirement: Weapon stick
An armed bite whose depth, widened to a double, is above 0.1 SHALL stick, its depth capped at 1. Above a promoted 0.4 the flags SHALL be 0x211000, else 0x11000. Strength SHALL be 12 times the depth and smoothing SHALL add 0.5. The count SHALL be 3 times the product of 83, the unit (the holder's link-iteration count) and the depth rounded to nearest, and the span twice that product. The weapon layer SHALL be 0xC0; the part, its parent and each child SHALL gain bit 0x40.

#### Scenario: A tenth stays free and a deep bite caps
- **WHEN** an unarmed depth of 1 is stuck, a depth just under 0.1 and a single 0.1 are stuck, a depth of 0.25 with smoothing 0.1 and unit 1 is stuck, a float depth of 0.4 with smoothing 0 and unit 1 is stuck, a depth of 1 and a depth of 3 are stuck, a depth of 0.11 with unit 2 is stuck, and a part layer of 3 with a parent of 0 and children 1 and 0x40 is marked
- **THEN** the unarmed bite and the depth under 0.1 stick nothing, the single 0.1 sticks, 0.25 uses flags 0x11000, strength 3, smoothing 0.6, count 63 and span 41.5, 0.4 uses flags 0x211000, count 99 and span bits 0x4284cccd, depth 1 uses count 249 and span 166, depth 3 matches that count, span and strength with smoothing 1, unit 2 uses count 54, the weapon layer is 0xC0, and the marked layers are 0x40, 0x43, 0x41 and 0x40

#### Scenario: A character's and a feral body's unit
- **WHEN** a depth of 0.25 is stuck with unit 20 and with unit 15, and a depth of 1.7 with unit 20
- **THEN** unit 20 gives count 1245 and span 830, unit 15 count 933 and span 622.5, and the capped depth count 4980, span 3320 and flags 0x211000

### Requirement: Held bite
A bite SHALL hold a weapon body's part to the struck part of another body by a grab of pull 1 and strain the stick span, at the contact on each part, its axis the strike's velocity in cm a tick in the weapon's frame. The solver link passes SHALL pull the two parts together. A break SHALL drop every grab of that weapon part on that part and clear bit 0x40 from the layer masks of the struck part, its neighbours and the weapon part.

#### Scenario: A tenth stays apart and a quarter closes
- **WHEN** a weapon body and a struck body 8 cm apart are solved with a bite of 0.25 at unit 20, then two such bodies are solved with no bite (a depth of 0.1 or less opens none)
- **THEN** the bite leaves them less than a quarter of the gap apart with less strain than it started with, and without a bite the gap stays where it started

#### Scenario: A spent bite breaks
- **WHEN** two bites of one weapon part on one part, each with strain 1, are solved 8 cm apart with the struck part's layer 0x40 and the weapon's 0xC0
- **THEN** both bites go, one break is recorded, the struck layer is 0 and the weapon layer 0x80
