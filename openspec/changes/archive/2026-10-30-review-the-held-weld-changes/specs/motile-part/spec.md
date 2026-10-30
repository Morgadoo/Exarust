## MODIFIED Requirements

### Requirement: Parry weld
A held weapon SHALL get a stiff disabled world weld at its origin, and a soft disabled weld of strength 0.05 only when its holder may parry: not in the heavy-claws state and without the creature bit (bit 0) in its body rule flags. The soft weld point SHALL be 0.3 times the top of the item's node box, in double, or the grip itself for the weapon class word 0x30.

#### Scenario: Holder in the heavy-claws state
- **WHEN** a holder in the heavy-claws state holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

#### Scenario: Creature holder
- **WHEN** a holder whose body rule flags carry the creature bit holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

### Requirement: Shield body words
A held shield's or torch's body SHALL take the held and mark bits (0x20002), lose the foot-sole bit, and take a link mobility equal to its contact mobility.

#### Scenario: Contact mobility 3.5
- **WHEN** a shield whose contact mobility is 3.5 is held
- **THEN** its link mobility is 3.5 and its sole bit is clear

## ADDED Requirements

### Requirement: Death weld stiffening
When the holder dies, each held item's soft weld SHALL take strength 1; no other grip link SHALL change, and a dead holder SHALL keep its held items.

#### Scenario: Corpse keeps its shield
- **WHEN** the holder of a shield dies
- **THEN** the shield's soft weld takes strength 1 and the shield stays held

### Requirement: Collapse weld slack
On the tick a holder starts collapsing, each item it still holds after the collapse lets go of its weapons SHALL have its soft weld's slack set to 10 cm, leaving the link's smoothed length and strength as they were. Each later collapse SHALL write it again.

#### Scenario: Collapsing with a shield
- **WHEN** a holder with a shield starts collapsing
- **THEN** the shield's soft weld has 10 cm of slack and strength 0.05

### Requirement: Collapse groan
A collapse's start SHALL ask the character's voice for line 2 with volume 0.15 + 0.1 × the second draw and pitch 0.8 + 0.1 × the first draw, each worked in double and narrowed once.

#### Scenario: Draws of 0.3 and 0
- **WHEN** the first draw is 0.3 and the second 0
- **THEN** the pitch is 0.83 and the volume 0.15
