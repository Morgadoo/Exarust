## MODIFIED Requirements

### Requirement: Energy casts
Shock SHALL start motion 0x27 and be made only when the motion starts, returning 1 either way; each tick its charge SHALL add the simulation's step milliseconds over 1200 (0.01 at 12 ms, firing on the 101st charging tick) and spend the cost times that times 1.2. Shock weapons SHALL need a weapon and a slot, arm at most four weapons, end a put-away weapon's arcs and arm a taken one while fewer than four are armed.

#### Scenario: A 12 ms step and a fifth weapon
- **WHEN** a 0.1 shock charges on 12 ms steps and four armed weapons see a fifth taken and one put away
- **THEN** each tick adds 0.01 and costs 0.0012, the burst fires about 1.2 s after the cast, the fifth is kept off, and the put-away weapon's arcs end
