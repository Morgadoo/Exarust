## MODIFIED Requirements

### Requirement: Step lift
An anchored stepping foot SHALL take the feet routine's lift −(1 − crouch)·f·(1.5 − the race's stance factor), with f = ((2 − r)·r·3 − 2)·s within 3s of its anchor (r = D²/(3s)², D the distance to the anchor) and 0.5·s beyond. The scale s SHALL be the lower leg's own lift scale, which nothing sets, so it is 0 and the stepping foot is not lifted. An unanchored foot's lift SHALL become 0.8 of itself each tick, and an anchored foot that is not stepping 0.9 of itself.

#### Scenario: A human step of 40 cm
- **WHEN** the scale is 16 cm, D is 40 cm, the crouch is 0 and the race's stance factor is 0.5
- **THEN** the lift is −11.5185 cm

#### Scenario: A stepping foot
- **WHEN** a foot steps 50 cm behind the chest on level ground
- **THEN** its lift stays 0 throughout the step

#### Scenario: Decay of a free foot
- **WHEN** an unanchored foot's lift is 10 cm
- **THEN** the next tick it is 8 cm
