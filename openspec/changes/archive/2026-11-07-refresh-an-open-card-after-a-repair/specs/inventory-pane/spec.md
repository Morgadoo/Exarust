## ADDED Requirements

### Requirement: Card refresh after a repair
After a repair's write and copy, an open card of that item SHALL be refreshed in place: its title and description SHALL be the record's name and description read again and rewritten by the apparel sentence for an apparel body, else by the weapon sentence over a weapon section whatever its model name, a crafted name taking the great table, and set without the condition blocks and pronoun markers. A record whose name does not resolve, or with neither section, SHALL leave the card as it is.

#### Scenario: Repaired trousers
- **WHEN** exanima01's cloth trousers (wear 160) have their card open and a repair kit is released on them
- **THEN** the card's sentence moves to the repaired wear, its Slash row and its value rise, without closing the card

#### Scenario: A crafted mace
- **WHEN** a weapon described "A mace. [sc]" at no damage is described by the builder and by the refresh
- **THEN** the builder's phrase comes from the flawless table and the refresh's from the great table

### Requirement: Card refresh rows
The refresh SHALL write an apparel card's coverage, impact, slash, crush, pierce and encumbrance rows (encumbrance 8 at 0xa1 or above) and its value, or a weapon card's striking rows from Slash's bar on for each non-zero side byte of side 0, its thrust and its value, from the record's bytes; a row the card does not have SHALL be skipped.

#### Scenario: Apparel rows
- **WHEN** an apparel card with only a Slash row and value 1 is refreshed from slash 11 and a value of 4 copper
- **THEN** its Slash row reads 0.55 and its value 4
