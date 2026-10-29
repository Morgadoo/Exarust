# Spec Delta

## MODIFIED Requirements

### Requirement: Parry weld
A held weapon SHALL get a stiff disabled world weld at its origin, and a soft disabled weld of strength 0.05 only when its holder may parry. The soft weld point SHALL be 0.3 times the top of the item's node box, in double, or the grip itself for the weapon class word 0x30. Death SHALL leave every grip link as it is, and a dead holder SHALL keep its held items.

#### Scenario: Holder in the heavy-claws state
- **WHEN** a holder in the heavy-claws state holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

#### Scenario: Corpse keeps its shield
- **WHEN** the holder of a shield dies
- **THEN** the shield's soft weld keeps its strength of 0.05 and the shield stays held

### Requirement: Unarmed hand mobility
When the equip refresh finds no weapon class in the first held item, each hand's contact mobility SHALL be its base times 0.5, or 0.25 with heavy claws; an armed character's hands SHALL keep their base. Every attack start in the heavy-claws state SHALL set both hands to their base times 0.25, and a collapse of an unarmed character SHALL put both back at their base.

#### Scenario: Heavy claws
- **WHEN** a hand's base is 2 and the race has heavy claws
- **THEN** its contact mobility is 0.5

#### Scenario: Unarmed collapse
- **WHEN** an unarmed human collapses
- **THEN** both hands' contact mobility is their base again

## ADDED Requirements

### Requirement: Foot thin-plate bit
The leg routine of every living tick that is not a collapse SHALL give each lower leg's first child the level mask's thin-plate bit (4), whatever its limit link. The collapse's start and death SHALL clear that bit and the strike-chain bit on those children only.

#### Scenario: Collapse
- **WHEN** a living body's collapse starts
- **THEN** its feet meet thin plates and cannot strike, and the collapsing ticks do not give the bit back

### Requirement: Gobbler collapse slack
When a gobbler (race index 8) starts to collapse, each lower leg's femur limit link SHALL take a slack of 10 cm.

#### Scenario: Other races
- **WHEN** a human collapses
- **THEN** its femur limit links keep their slack

### Requirement: Shield body words
A held shield's body SHALL take the held and mark bits (0x20002) and a link mobility equal to its contact mobility.

#### Scenario: Contact mobility 3.5
- **WHEN** a shield whose contact mobility is 3.5 is held
- **THEN** its link mobility is 3.5

### Requirement: Revive
A revive SHALL act only on a dead character: it is knocked out at once (condition down, the wake nibble 0x30, the wake clock now), its yellow health becomes half its red, and its hands are set up unarmed again when the record's weapon slots are empty. Its body SHALL mark every limb below the pelvis muscled with its base contact mobility, give each foot the thin-plate bit and the 0x30 layer, turn the muscles on with tone and stance 0 and the living pin gain, and keep death's link-iteration count.

#### Scenario: Red 0.3
- **WHEN** a dead character with red health 0.3 is revived
- **THEN** its yellow health is 0.15 and it is down

#### Scenario: Not dead
- **WHEN** a collapsing character is revived
- **THEN** nothing changes
