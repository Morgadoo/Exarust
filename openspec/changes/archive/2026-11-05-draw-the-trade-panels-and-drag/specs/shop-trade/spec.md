# Spec Delta

## ADDED Requirements

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
