## MODIFIED Requirements

### Requirement: Held item body
An equipped item whose model has a rigid root SHALL be its own solver body, tied to its holder by the grip links. Whether the holder lives or not, its pivot links SHALL use the limp link law (pivot softening 0.001 per cm squared), with the item and the hand each moved by their share of the mobility sum. The item SHALL never meet its holder's body or another item the same holder holds. It SHALL be freed, with friction 0.75 and its own group masks, when its holder loses its drive.

#### Scenario: Held at the grip
- **WHEN** a held item's body and its holder's body tick together for many solver ticks
- **THEN** the item stays at its grip point on the hand

#### Scenario: Holder falls
- **WHEN** the holder's body goes limp
- **THEN** the item's links stop acting and the item falls free

#### Scenario: Living holder's hand pivot
- **WHEN** a living holder's hand pivot has a 10 cm gap
- **THEN** the gap closes by 1/1.1, as for a limp holder


## ADDED Requirements

### Requirement: Held item world weld law
A held item's world weld (a grip link without the pivot flag) SHALL move the item alone, past its slack, by strength times the gap, cut to a 10·strength² cm move once the gap exceeds 10·strength cm, or by strength times the excess for a link with slack, whether the holder lives or not.

#### Scenario: Soft weld 20 cm away
- **WHEN** an enabled soft world weld of strength 0.05 without slack has a 20 cm gap
- **THEN** the item moves 0.025 cm toward its anchor

### Requirement: Held item drop words
A dropped held item SHALL lose only its strike, chain and mark bits (0x30001), keeping its other state bits, and take friction 0.75.

#### Scenario: Dropped torch
- **WHEN** a held torch with the held and mark bits is dropped
- **THEN** its mark bit clears and its held bit stays

### Requirement: Torch friction
A held torch's body SHALL have friction 0, while a held weapon's or shield's has 0.5.

#### Scenario: Torch in hand
- **WHEN** a torch is held
- **THEN** its body's friction is 0

## REMOVED Requirements

### Requirement: Death weld stiffening
**Reason**: The death routine's strength-1 loop walks the leg parts kept for the feet and writes their limit links, not a held item's soft weld.
**Migration**: The legs' write is `Ragdoll::go_limp`; held items keep their welds through death.

### Requirement: Collapse weld slack
**Reason**: The collapse's 10 cm slack is a gobbler's femur limit links, walked from the same leg-part list, not a held item's soft weld.
**Migration**: The femur write is `Ragdoll::slacken_femur_limits` ("Gobbler collapse slack"); held items keep their welds through a collapse.
