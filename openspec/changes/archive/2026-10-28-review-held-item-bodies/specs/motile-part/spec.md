## ADDED Requirements

### Requirement: Held item body
An equipped item whose model has a rigid root SHALL be its own solver body, tied to its holder by the grip links. These links use the limp link law (pivot softening 0.001 per cm squared), with the item and the hand each moved by their share of the mobility sum. The item SHALL never meet its holder's body or another item the same holder holds. It SHALL be freed, with friction 0.75 and its own group masks, when its holder loses its drive.

#### Scenario: Held at the grip
- **WHEN** a held item's body and its holder's body tick together for many solver ticks
- **THEN** the item stays at its grip point on the hand

#### Scenario: Holder falls
- **WHEN** the holder's body goes limp
- **THEN** the item's links stop acting and the item falls free

### Requirement: Grip link build
The grip builder SHALL make the main links from the table's points scaled by the body scale, mirrored in x and z when flipped, with the table's slack and strength 1, and 0.01 smoothing on the second link. A grip with a second hand SHALL add that hand's link, with points scaled by the character's size and the item point's y scaled by the record's second-hand factor. It SHALL also add an arm link whose hand-side point is the table's raw point. Every link left at zero smoothing SHALL take 0.001.

#### Scenario: Arm point
- **WHEN** a two-handed grip is built for a character of size 1.5
- **THEN** the arm link's hand-side point is the table's point unscaled

### Requirement: Grip mode store
The grip mode setter SHALL store the requested mode before running the grip's callback, and SHALL mark the links as relaxing only when the grip has a callback. The two-hander's half-sword turn of mode 2 into mode 1 SHALL be local to the callback.

#### Scenario: Half-sword thrust
- **WHEN** mode 2 is asked for a half-sword thrust grip
- **THEN** the links take mode 1's arrangement and the stored mode is 2

### Requirement: Weapon body impact
A striking held item's contact with a non-limb part SHALL count on the item and run the hard impact on the owner's swing. A stab or an overhead records round(5 times the square root of the impact) as its strength, with a zero stall. The item SHALL then sound only when its own sound clock is below the game clock, at volume 0.005 times the impact plus 0.3 times the square of a hundredth of the impact, at most 1. The cooldown SHALL be 200 minus round(80 times the volume) ms.

#### Scenario: Struck weapon rings
- **WHEN** a striking weapon body in an overhead is hit hard by a thrown body
- **THEN** it plays its set, records the overhead's strength and stamps its sound clock

### Requirement: Held reach
With hands bits in the held item's class, its reach SHALL be the record byte times 0.01 times the item's y extents, worked in double and rounded to single, plus 60 cm. Without them, the race's reach SHALL be used. The aim height SHALL be 0.9 times the extents, worked in double.

#### Scenario: Double product
- **WHEN** the byte is 5 and the extents are 97.1 cm
- **THEN** the reach is 64.855 cm (single arithmetic would give 64.854996)
