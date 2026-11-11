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
The running game's shop world SHALL read the object database's table once, taking each record's id, the table's flags word as its kinds and its shop fields, and SHALL make apparel and weapon copies in the live object database. A copy's source SHALL be its header's parent word, and a copy dropped from the shelf SHALL be removed from its tier. Without a base tier the copies SHALL be stand-in ids under the instance bits that remember their source.

#### Scenario: Table flags
- **WHEN** a database record's table flags word is 0x4000 and its payload holds a model and a name section
- **THEN** the entry's kinds are 0x4000 and its rarity is the model's

#### Scenario: Stand-in copies
- **WHEN** an apparel and a weapon copy are made from records 10 and 20 with seed 7 and no base tier
- **THEN** they are the first and second instance ids, their sources are 10 and 20, and releasing the first forgets only its source

### Requirement: Live apparel copy
An apparel copy SHALL be the record's instance (or the record itself when it has no instance bit). Its body's condition byte SHALL take a quality `2u − 1` from the shared seed, with damage 0x40 and dirt 0x35 and their spreads; the apparel palette SHALL be rolled for the body's condition plus quality; then the copy SHALL be refreshed from its template: value word, paint seed when it has none, protection bytes, the layers' and variant's roll, the condition pass.

#### Scenario: An apparel copy's own body
- **WHEN** marked apparel record 10 is copied with seed 7
- **THEN** the copy is local 1 naming 10, its condition byte is the one seed 7 alone gives, and record 10's body is unchanged

### Requirement: Live weapon copy
A weapon SHALL be copied only when its header word has the instance bit and its model is a factory's (`@…`). The copy SHALL be graded at level 1 and refreshed from its template: value word, template bytes, the condition's roll, the factory's stats, the condition pass. Any other weapon SHALL be stocked as itself.

#### Scenario: A weapon copy
- **WHEN** a marked `@sword_x` record and an unmarked plain weapon are copied
- **THEN** the first is local 1 with its grade's damage and dirt drawn and the factory `sword_x` asked, and the second is stocked as itself

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

### Requirement: Trade window item panels
The trade window's pack, sell basket, buy basket and stock SHALL be item panels in the page's client, inside its border of 8: the pack at (0, 0x180) 0xe4 by 0xb4, the sell basket at (0xe8, 8) and the buy basket at (0x1b0, 8), each 0xc4 by 0x174, and the stock 0xe4 by 0x234 with its right edge at the client's. They SHALL show each item as its icon, with no rows or names.

#### Scenario: The stock's height
- **WHEN** the trade window opens
- **THEN** the stock fills the page's height less the border at the top and bottom

### Requirement: Trade window panel flags
The pack and the sell basket SHALL take only items without the shop's flag 0x80000000, and the buy basket and the stock only items with it.

#### Scenario: A shop item dropped on the sell basket
- **WHEN** a stock item is dragged onto the sell basket
- **THEN** the basket refuses it and the item stays in the stock

### Requirement: Trade window layout
Opening the trade window SHALL load the pack from the player's inventory list with its stored places and flags, and the stock from the shelf's list, each item with the shop's flag, laid out by the item panel's seeded grid pass with the day counter plus one as the seed. The baskets SHALL start empty.

#### Scenario: Stock flags
- **WHEN** the shop opens on a shelf of two items
- **THEN** both stock entries carry 0x80000000

### Requirement: Trade window drag
A left press over an item of a trade window panel SHALL start its drag; the release over a panel that accepts it SHALL move it there at the pointer less the grab, over its own panel SHALL only move it, and anywhere else or over a panel that refuses it SHALL leave it where it was. While it is dragged its icon SHALL follow the pointer.

#### Scenario: Buying by drag
- **WHEN** a stock item is dragged onto the buy basket
- **THEN** it leaves the stock, shows in the basket and the buy total sums its value word

### Requirement: Basket totals
Each basket's change SHALL sum the value words of the items it holds, the sell basket's then quartered, and redo the bottom row.

#### Scenario: An item dragged back
- **WHEN** the buy basket's only item is dragged back to the stock
- **THEN** the buy total is 0

### Requirement: Trade window preview
The trade window's character preview SHALL show a copy of the player's record. An item of any of its panels released over the preview SHALL be worn (apparel, after the equip conflict pass, refused by a locked piece) or taken into the hands (a weapon or a shield) on the copy, and leave its panel; a piece the conflict pass takes off that is neither the braies nor flagged `0x80000000`, and a hand item the take-in moves out that is not so flagged, SHALL go into the pack at its centre.

#### Scenario: A tunic from the pack
- **WHEN** a tunic is dragged from the pack onto the preview of a player in braies
- **THEN** the copy wears the braies and the tunic and the pack no longer holds the tunic

### Requirement: Trade window preview take-off
A left press over the preview's figure SHALL pick the item under the pointer and drag it, its icon following the pointer. Its release over a panel that accepts it SHALL put it there and take it off the copy (a worn piece leaves the worn list and the underwear rule runs, a held item leaves its hand); an item flagged `0x80000000` SHALL come off wherever its drag ends, any other only when a panel took it.

#### Scenario: The tunic back to the pack
- **WHEN** the worn tunic is dragged off the preview onto the pack
- **THEN** the pack holds it and the copy wears only the braies

### Requirement: The merchant's options
The trade window SHALL add the merchant's options form only when it trades with the merchant: `Look for:` over Anything, Clothing & Armour and Weapons, and `Rank:` over Any and the six ranks. Opening it SHALL select the row of the merchant's stock type (0, 0x1000, 0xc000), keeping the last row when the type is not listed, and the specialisation plus two. A pick SHALL set the stock type to the list's word for the row, or the specialisation to the row less two.

#### Scenario: Weapons of Novice rank
- **WHEN** a new career's merchant trade opens and Weapons, then Novice, are picked from the lists
- **THEN** the boxes first read Anything and Any, and then the stock type is 0xc000 and the specialisation 2

### Requirement: The merchant's options take keys
Keys typed over a merchant option box SHALL edit the box's shown text only; Enter SHALL re-store the stock type or specialisation from the box's selection word.

#### Scenario: Enter after editing
- **WHEN** the Look for: box's text is edited and Enter pressed on a new career
- **THEN** the stock type is 0, the Anything row's
