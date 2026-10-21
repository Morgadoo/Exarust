## MODIFIED Requirements

### Requirement: Giving and having
Giving an item to a character with no open pack SHALL append it at the pack's origin; with the character's view open its pack panel SHALL take it at its centre and place it, and an item the panel refuses SHALL be lost. Giving back an item taken out of a hand SHALL list it at 40 by 4 sixteenths of a unit. Asking whether a character has an item SHALL search its list and every container in it, at any depth.

#### Scenario: An item in a pouch in a bag
- **WHEN** a character carries a bag holding a pouch holding an item
- **THEN** it has the bag, the pouch and the item, and not an item none of them holds

#### Scenario: A refused gift
- **WHEN** an item without an icon is given while the character's view is open
- **THEN** the panel refuses it and the carried list does not gain it
