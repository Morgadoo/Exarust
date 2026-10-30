# Spec Delta

## ADDED Requirements

### Requirement: Bite opened by a wound
A wound on flesh SHALL open a bite when its bite depth, widened to a double, is above 0.1 and the striker's weapon class has a hands bit (0x30): one- and two-handed weapons, polearms and dual wielding. Fists, claws and an empty class SHALL never bite, and a turned, clashed or blocked blow SHALL open none. The bite SHALL carry the contact, the strike's velocity in cm a tick, the depth and the zone smoothing, and its unit SHALL be the holder's link-iteration count.

#### Scenario: A deep cut by a held weapon
- **WHEN** a sword cut of 20 cm a tick at 70 cm along the blade wounds bare flesh, struck by a one-handed weapon class and then by the fists' class
- **THEN** the weapon's cut asks for one bite with the blow's depth and smoothing and the velocity times 100/83, and the fists' asks for none

#### Scenario: Weapon classes
- **WHEN** a depth of 0.5 is dealt by the classes 0x10, 0x20, 0x30, 0x90, 0x00, 0x80 and 0xC0
- **THEN** the first four bite and the last three do not
