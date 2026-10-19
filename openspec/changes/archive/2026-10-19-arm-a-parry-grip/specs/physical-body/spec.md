# Spec Delta

## ADDED Requirements

### Requirement: Armed grip
A cleave SHALL be refused. Inside 80 cm the falloff SHALL be 1, at 165 cm one half, and at 250 cm and beyond 0. Handling 0.5 inside 80 cm SHALL have speed 0.5, blend 10, and strength (1.4 minus handling) times the falloff times 0.75 times 0.6. A passed blow SHALL square the handling and disable the grips. A shield of weight 0.8 and stamina 0.5 SHALL have speed 0.8, blend 16 and flags 0x800. Slot 1 SHALL be used only while attacking outside motions 8 to 10 with two weapons held.

#### Scenario: A close weapon, a passed blow, and a shield
- **WHEN** a cleave is gripped, handling 0.5 is gripped at 40 cm, at 165 cm, at 250 cm, and at 251 cm, a passed blow is gripped, a shield of weight 0.8 and stamina 0.5 is gripped, and an attack of motion 5 with two weapons, motion 9 with two weapons, and motion 5 with one weapon are gripped
- **THEN** the cleave is refused, 40 cm has speed 0.5, blend 10, and strength bits 0x3ecf5c29, 165 cm has strength bits 0x3e4f5c29, 250 cm and 251 cm have strength 0, the passed blow has speed 0.25, blend 5, and flags 0x100000, the shield has speed bits 0x3f4ccccd, blend 16, strength bits 0x3df5c290, and flags 0x800, and the slots are 1, 0, and 0
