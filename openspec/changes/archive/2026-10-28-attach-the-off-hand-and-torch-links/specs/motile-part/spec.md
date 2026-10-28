## MODIFIED Requirements

### Requirement: Parry weld
A held weapon SHALL get a stiff disabled world weld at its origin, and a soft disabled weld of strength 0.05 only when its holder may parry. The soft weld point SHALL be 0.3 times the top of the item's node box, in double, or the grip itself for the weapon class word 0x30. The death stiffening SHALL set a soft weld at a positive index to strength 1, and a dead holder SHALL keep its held items.

#### Scenario: Holder in the heavy-claws state
- **WHEN** a holder in the heavy-claws state holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

#### Scenario: Corpse keeps its shield
- **WHEN** the holder of a shield dies
- **THEN** the shield's soft weld takes strength 1 and the shield stays held

## ADDED Requirements

### Requirement: Off-hand item
A shield SHALL be held by a back pivot at (−20, 0, 0) cm to its hand's forearm, two face pivots at (5, 0, ±15) cm to the left hand (the forearm-frame point less the left hand's rest translation) and a soft weld of strength 0.05 at its origin. A second weapon SHALL take the grip table's off-hand entry and strike from the second slot.

#### Scenario: Left-held shield
- **WHEN** a shield is held in the left hand
- **THEN** its face pivots join the left hand, not the right

### Requirement: Torch links
A held torch SHALL be joined to the hand holding it by two pivots at (0, −20, 0) and (10, 0, 0) cm and carry a soft weld of strength 0.05 at its origin.

#### Scenario: Off-hand torch
- **WHEN** the torch is held in the left hand
- **THEN** both pivots join the left hand

### Requirement: Flip command gate
A flip request SHALL be refused while the holder is switching (condition 0x10000) or when the first held item's section has a zero flipped-side kind byte, and SHALL turn only the first held item.

#### Scenario: No flipped side
- **WHEN** a flip is requested for a weapon whose flipped-side kind is 0
- **THEN** no flip starts

### Requirement: Collapse release
A collapsing holder SHALL let go of its weapons and its torch and keep its shield.

#### Scenario: Knocked down with a sword and shield
- **WHEN** the holder collapses
- **THEN** the sword is released and the shield stays

### Requirement: Limit link
A living body's link without the pivot flag SHALL move its part alone by strength·d once |d|² (cm²) exceeds the slack, capped at 10 cm, or 2 cm with the vertical gap zeroed and only in the first two iterations for a horizontal link. A parented part's skeleton links SHALL be pivot links.

#### Scenario: Shipped skeleton
- **WHEN** a character body is built from humanoid.rsq
- **THEN** none of its links is a limit link
