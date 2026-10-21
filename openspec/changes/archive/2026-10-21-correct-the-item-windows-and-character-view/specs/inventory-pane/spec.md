## MODIFIED Requirements

### Requirement: Panel entries
A panel entry SHALL not be drawn while dragged, held or without an icon; it SHALL be tinted green by the green mask, yellow at half opacity by the yellow mask alone, green at half opacity by both, else black.

#### Scenario: The tints
- **WHEN** entries flagged with the green mask, both masks, the yellow mask and neither are drawn, and a held one
- **THEN** they are green, half-opaque green, half-opaque yellow and black, and the held one is not drawn

### Requirement: Item GUI controls
Rating dots SHALL be whole up to the value less a quarter, with a half dot for a remainder above a quarter. An icon's halo SHALL make each inner pixel's alpha the clamped sum of the five-pixel cross around it. A popup SHALL open centred on the cursor and kept on the screen, its bottom 8 units below the cursor.

#### Scenario: Three and a half dots
- **WHEN** a rating of 3.5 is drawn at unit 4
- **THEN** three dots go 12 pixels apart and a half dot follows

## ADDED Requirements

### Requirement: Looted worn panel
A looted body's worn panel SHALL leave out every worn item or held shield the icon cache cannot draw, without drawing random numbers for it, scatter the rest over the middle half of the panel (a quarter plus a draw below half of each side) and push them apart over four rounds, measuring both axes with the icons' horizontal half extents and keeping every centre inside.

#### Scenario: An item without an icon
- **WHEN** a looted body wears two items and the cache cannot draw the first
- **THEN** the panel holds only the second, placed where it would be had the body worn it alone

### Requirement: Socket rows
A crystal dropped into a socket SHALL replace the one there, which goes back with its charge; a drag from the same socket SHALL be refused, and a finished drag SHALL clear the socket's kind and keep its charge byte. Committing a socket row SHALL write it into the item's weapon section, else its apparel section, and an AR crystal SHALL fire the game's fourth function.

#### Scenario: A socket row's commit
- **WHEN** a row holding the AR crystal is committed into an item with a weapon and an apparel section, then one with only apparel, then one with neither
- **THEN** the first goes to the weapon section and fires the function, the second to the apparel section, and the third nowhere

### Requirement: Weapon handling
A weapon card SHALL show weight w·0.01, balance b·0.05 and handling 5·(a + f) with a = (g − 127)·0.01 and f = ((1 − b·0.01)·0.6 + w·0.004 + 0.1)/1.04 for a balance byte up to 100 or (w·0.004 + 0.1)/1.04 above, each product in double precision narrowed to single as the original computes it; a weapon flagged 0x6000000 SHALL show weight 1, handling 5 and balance 0. The striking side's shown rows SHALL fill the card from the first in slash, crush, pierce order at byte · 0.05.

#### Scenario: A balanced weapon
- **WHEN** a weapon of weight 100, balance 50 and grip 127 is shown
- **THEN** its weight is 1, its balance 2.5 and its handling about 3.846

### Requirement: Card condition blocks
A card's description SHALL first take its condition block: blocks start at an at sign with a two-digit limit five and six bytes after it (00 meaning 100) and text from eight bytes after it to three bytes before the next at sign, and the first block whose limit reaches the condition byte's percentage SHALL replace the text from the first block on.

#### Scenario: A worn item's text
- **WHEN** a text with a 50 block and a 00 block is shown for condition bytes 128 and 255
- **THEN** the first shows the 50 block's text and the second the 00 block's

### Requirement: Card pronouns
A card's pronoun markers SHALL be replaced by the name in their last field: HeShe, HimHer, HisHer or HisHers give he or she, him or her, his or her, his or hers by the owner's sex; an unknown name SHALL give nothing; the word SHALL be capitalised at the start of the text or two bytes after a dot.

#### Scenario: A sentence start
- **WHEN** a marker naming HeShe follows ". " for a female owner
- **THEN** it reads "She"
