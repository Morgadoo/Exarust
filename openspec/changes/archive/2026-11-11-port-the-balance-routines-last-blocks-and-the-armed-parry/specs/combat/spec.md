## ADDED Requirements

### Requirement: Armed parry link
While an armed parry holds, the parry link of the weapon in its slot (the item's soft world weld; the holder's right-hand item first) SHALL become a plain world link to the parry point with the grip's strength, and the other hand's item's link SHALL be disabled. A blow that has passed SHALL disable every item's link. The shield's plane link is not modelled.

#### Scenario: Two weapons
- **WHEN** an armed parry holds with strength 0.3 at the point (100, 200, 300) cm in slot 0
- **THEN** the right-hand item's link holds to (1, 2, 3) m at 0.3 and the left-hand item's is disabled

### Requirement: Parry link release
The parry's end SHALL disable every held item's parry link and leave the hand grips as they are. An unarmed parry SHALL write no link.

#### Scenario: The end
- **WHEN** a parry that held an item's link ends
- **THEN** that link is disabled and its hand grip stays enabled

### Requirement: Stab handling pushes
A stab past clock 0.1 (a double compare) that does not cleave SHALL push the item body at the point 0.9 up its blade and the right hand by the handling pushes along the guide from the frame's anchor to the aim point, in the original's float order.

#### Scenario: Half the clip
- **WHEN** the clock is 0.5 with a balance of 0.5, the weapon point (3, 4, 5) cm and the hand (1.5, 2, 2.5) cm from the anchor and the guide up +z
- **THEN** the weapon's push is about (−6.586, −8.782, 17.472) cm and the hand's (−0.274, −0.366, 1.092) cm
