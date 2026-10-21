## ADDED Requirements

### Requirement: Possessions from the record
A character's possessions SHALL start from its record: the profession word with its second-set flag 0x0100_0000, the four held items with their flags words (the out set's main and off hand, then the stowed set's), the worn items, and the carried list with each entry's position in sixteenths of a unit.

#### Scenario: A lit torch in the off hand
- **WHEN** a record holds items 1 to 4 in its hands with the off hand's flags 0x0200_0000, the second-set flag in its profession word, and a held item listed at 40 by 4
- **THEN** the out set holds 1 and a lit 2, the stowed set 3 and 4, the second set is out, and the entry sits at 40 by 4
