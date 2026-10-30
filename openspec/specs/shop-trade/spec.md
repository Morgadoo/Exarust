# shop-trade Specification

## Purpose
The arena shop's shelf as the original keeps it: the first stock and its daily rotation, the candidates and their rarity weights, the draw and the new copies, the merchant's options, and the transaction a confirmed trade makes.

## Requirements

### Requirement: Shop stock
An empty shelf SHALL draw 18 to 20 items. A shelf under 20 SHALL drop 2 to 5 of its oldest items and gain 4 to 7, one under 24 drop and gain 4 to 7, and a larger one drop 4 to 7 and gain 2 to 5. Dropped instances SHALL be released.

#### Scenario: Rotation
- **WHEN** 18 items are on the shelf and the roll steps from 11
- **THEN** the two oldest are released and five are added, 21 in all

### Requirement: Stock candidates
A candidate SHALL be an object database item of the asked kinds (apparel, weapons and shields when none are asked) that is listed and has a named model with both size words set and, when a rank is asked, that rank. Asking those three kinds by name for rank 5 SHALL ask rank 4.

#### Scenario: Weapons of a rank
- **WHEN** weapons and shields of rank 1 are asked from a table whose only rank-1 arm is a weapon
- **THEN** every item drawn is a copy of that weapon

### Requirement: Stock weights
An item of rarity `r` hundredths SHALL weigh `round(10 · (10s)^(1+s))` with `s = 1 − r/100` by the original's quick logarithm and exponential, and SHALL not be drawn unless `s` is above 0.005. Each copy of the item already on the shelf SHALL divide its weight by 3, and weapons and shields SHALL halve it.

#### Scenario: Weights
- **WHEN** rarities 0, 10, 20, 50, 80, 90, 99 and 100 are weighed
- **THEN** they weigh 1000, 650, 422, 112, 23, 10, 1 and nothing

### Requirement: Stock draw
Each draw SHALL step the roll by `× 0x08088405 + 1` and take the first candidate whose running weight exceeds the high half of the total times the roll; no such candidate SHALL lower the count. Apparel and weapons SHALL be stocked as new copies and anything else as its id. A new apparel copy SHALL take a quality `2u − 1` from the variant roll into its condition byte, damage 0x40 and dirt 0x35.

#### Scenario: A held item is shunned
- **WHEN** sixteen copies of the heaviest item are on the shelf at a restock
- **THEN** the new items are the weapon and the shield

### Requirement: Merchant options
The merchant's category options SHALL be Anything, Clothing & Armour and Weapons, asking all kinds, apparel and weapons with shields; the rank options SHALL be Any, Inept, Aspirant, Novice, Adept, Expert and Master, option `n` asking rank `n − 2`.

#### Scenario: Options
- **WHEN** the third category and the first rank are chosen
- **THEN** weapons and shields of any rank are asked

### Requirement: Transaction
A confirmed trade SHALL take its due from the company's coins, leaving none under 1 copper, add its two tallies, append the items sold to the shelf in order and remove each item bought at its first place.

#### Scenario: Paying 100
- **WHEN** a company of 343 copper pays 100 for an item and sells another
- **THEN** it holds 243, the bought item leaves the shelf and the sold one ends it

### Requirement: Trade window
The trade window SHALL be a 0x36c × 0x244 page titled "Trade" with the character preview and the player's pack, the quartered sell basket, the full-price buy basket and the stock at the page's right edge. Its bottom row SHALL show "Pay:" and the difference or "Receive:" and its negation; a purse short of the payment SHALL hide the label, warn "You cannot afford this transaction." and disable the Trade button, which paying with both baskets empty SHALL also disable.

#### Scenario: A short purse
- **WHEN** the buy basket's total less a quarter of the sell basket's exceeds the purse
- **THEN** the warning shows and the Trade button is disabled

### Requirement: Live shop world
The running game's shop world SHALL read the object database's table once, taking each record's id, the table's flags word as its kinds and its shop fields, and SHALL make apparel and weapon copies as new instance ids that remember their source.

#### Scenario: Table flags
- **WHEN** a database record's table flags word is 0x4000 and its payload holds a model and a name section
- **THEN** the entry's kinds are 0x4000 and its rarity is the model's

#### Scenario: Copies
- **WHEN** an apparel and a weapon copy are made from records 10 and 20 with seed 7
- **THEN** they are the first and second instance ids, their sources are 10 and 20, and releasing the first forgets only its source

### Requirement: Player basket
A confirmed trade SHALL append the items the player takes to the player's basket and remove each item given at its first place, passing over one the basket does not hold, then report the basket's duplicate pairs.

#### Scenario: Take and give
- **WHEN** a basket of 7 and 0x8000_0002 takes 9 and 0x8000_0002 and gives 7 and 5
- **THEN** the basket is 0x8000_0002, 9, 0x8000_0002 with duplicate pair (0, 2)

### Requirement: Apparel quality share
The share an apparel copy's description phrase reads SHALL be its condition byte's low two bits plus the 5-bit field in bits 2 to 6 times 1/31 in double precision, negated when bit 7 is set.

#### Scenario: Negative quality
- **WHEN** the byte holds condition 1, quality 2 and bit 7
- **THEN** the share is 1 − 2/31 as a single

### Requirement: Shop window re-layout
The shop window's layout SHALL bind the stock list to its panel with the list's number when one is supplied. A preview pick that finds an item SHALL lay the window out again on that item and answer false; no item SHALL answer true.

#### Scenario: A pick
- **WHEN** the preview picks item 4, then nothing
- **THEN** the first lays the window out on item 4, the second changes nothing
