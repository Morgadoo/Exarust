## MODIFIED Requirements

### Requirement: Step lift
An anchored stepping foot SHALL lift by −(1 − crouch) · f · (1.5 − the race's stance factor), whatever stance factor the body's gait or a knock-out has set. s SHALL be 0.4 times the horizontal distance to the anchor and no more than 30 cm. r SHALL be D²/(3s)², D the distance to the anchor. f SHALL be ((2 − r)·r·3 − 2)·s when D is at most 3s, and 0.5·s otherwise. An unanchored foot's lift SHALL become 0.8 of itself each tick, and an anchored foot that is not stepping 0.9 of itself.

#### Scenario: A human step of 40 cm
- **WHEN** the horizontal distance and D are both 40 cm, the crouch is 0 and the race's stance factor is 0.5
- **THEN** the lift is −11.5185 cm

#### Scenario: Decay of a free foot
- **WHEN** an unanchored foot's lift is 10 cm
- **THEN** the next tick it is 8 cm
