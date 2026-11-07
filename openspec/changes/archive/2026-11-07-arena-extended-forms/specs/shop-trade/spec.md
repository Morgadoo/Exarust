## ADDED Requirements

### Requirement: The merchant's options
The trade window SHALL add the merchant's options form only when it trades with the merchant: `Look for:` over Anything, Clothing & Armour and Weapons, and `Rank:` over Any and the six ranks. Opening it SHALL select the row of the merchant's stock type (0, 0x1000, 0xc000), keeping the last row when the type is not listed, and the specialisation plus two. A pick SHALL set the stock type to the list's word for the row, or the specialisation to the row less two.

#### Scenario: Weapons of Novice rank
- **WHEN** a new career's merchant trade opens and Weapons, then Novice, are picked from the lists
- **THEN** the boxes first read Anything and Any, and then the stock type is 0xc000 and the specialisation 2
