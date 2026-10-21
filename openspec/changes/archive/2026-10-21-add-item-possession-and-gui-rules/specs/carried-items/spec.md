## ADDED Requirements

### Requirement: Weapon set swap
A character's possessions SHALL hold an out and a stowed weapon set of a main and an off hand, and a carried list in which an item in a hand stays listed with its held bit 0x100. Swapping the sets SHALL toggle the record's second-set flag 0x0100_0000, unmark each going item's held entries once per hand it fills less the incoming hands that hold it, mark the incoming items held the same way, and trade the sets. A two-handed item SHALL be counted once, from the main hand.

#### Scenario: A sword for a two-handed axe
- **WHEN** a sword is out and a two-handed axe fills both stowed hands, and the sets are swapped and swapped back
- **THEN** after the first swap the sword's entry is unmarked, both axe entries are held and the flag is set; after the second the marks and the flag are as before

#### Scenario: An item in both sets
- **WHEN** the same item is in the out main hand and the stowed main hand
- **THEN** a swap keeps its held mark and the main hand does not change

### Requirement: Hands after a swap
The hands SHALL take what changed: a weapon as a weapon, a light weapon in the off hand as a light, a shield as a shield, another item with a light section as a light, anything else as a weapon. Only a weapon SHALL take the stored lit and flipped bits.

#### Scenario: A torch in the off hand
- **WHEN** the incoming set holds a flipped weapon in the main hand and a lit torch in the off hand
- **THEN** the main hand takes a flipped weapon and the off hand a burning light

### Requirement: Giving and having
Giving an item to a character with no open pack SHALL append it at the pack's origin; giving back an item taken out of a hand SHALL list it at 40 by 4 sixteenths of a unit. Asking whether a character has an item SHALL search its list and every container in it, at any depth.

#### Scenario: An item in a pouch in a bag
- **WHEN** a character carries a bag holding a pouch holding an item
- **THEN** it has the bag, the pouch and the item, and not an item none of them holds

### Requirement: Taking one-handed items
A one-handed item SHALL go to a free main hand. With the main hand full, a light beside one weapon or the fists SHALL go to the off hand as a light, freeing it first; a light already in the main hand SHALL move to a free off hand, else be taken out; a second weapon SHALL go to the off hand only when asked for and never beside a two-hander; otherwise the main hand's item SHALL be taken out.

#### Scenario: A torch and a sword
- **WHEN** a sword is taken while a torch is in the main hand and the off hand is free, then a torch is taken beside a sword
- **THEN** the first torch moves to the off hand and the sword goes to the main hand; the second torch goes to the off hand as a light

### Requirement: Two-handed items and taking out
A two-handed item SHALL free the off hand, then the main hand. Taking an item out SHALL read and clear its hand's record, and a dropped first weapon SHALL hand the main hand to an off-hand weapon or light.

#### Scenario: Dropping the sword
- **WHEN** the first weapon is dropped while the off hand holds a dagger
- **THEN** the record's main hand holds the dagger and the off hand is empty

### Requirement: Loose items and cores
A held item coming loose SHALL clear every out hand that is not a shield and its first held entry, and SHALL clear a stowed hand whose item is no longer listed. A race flagged 0x2000 SHALL take only the AL power crystal as its core, one flagged 0x1000 only crystal A, and only while it wears none of the five power crystals.

#### Scenario: A golem's core
- **WHEN** a body of race flags 0x1000 is given crystal A, crystal AL, then crystal A while wearing crystal C
- **THEN** it takes the first and refuses the other two
