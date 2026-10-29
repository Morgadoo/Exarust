## ADDED Requirements

### Requirement: Hub machine side
The side an ArenaMach click compares SHALL be the x of the clicked node's world bounding centre, and only a side above zero SHALL open the board; zero, negative zero, a negative side and NaN SHALL open Manage Company.

#### Scenario: Centre, not origin
- **WHEN** ArenaMach nodes with their origin at x 5 and bounding centres at x −10 and at x 10 are clicked
- **THEN** the first opens Manage Company and the second the board

### Requirement: Hub first stock
A hub click on the shop or on the merchant whose shelf is empty SHALL build that shelf's first stock from the object database: the shop with the default kinds and any rank, the merchant with its stock type and specialisation. A shelf that has items SHALL be left as it is.

#### Scenario: One listed apparel record
- **WHEN** the live stock world holds one listed apparel record of rarity 0 and the shop is clicked twice
- **THEN** the first click shelves 18 to 20 copies of that record and the second leaves the shelf unchanged

### Requirement: Merchant chunk words
The save's merchant chunk SHALL load and write the merchant's stock type and specialisation as the career's, 0 and −1 in a new career.

#### Scenario: Round trip
- **WHEN** a career with a weapons merchant (stock type 0xc000) of rank 3 is written and applied to a new career
- **THEN** the chunk holds 0xc000 and 3 and the new career's merchant has them
