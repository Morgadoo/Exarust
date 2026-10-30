# Spec Delta

## ADDED Requirements

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
