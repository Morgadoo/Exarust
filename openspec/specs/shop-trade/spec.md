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
