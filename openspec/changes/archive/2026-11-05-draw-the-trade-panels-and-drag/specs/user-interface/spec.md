# Spec Delta

## MODIFIED Requirements

### Requirement: Trade window item cards
A right press over an item of the trade window's stock or buy basket SHALL ask for that item's card with the flags `0x80000000` (the card then shows its whole value), and over an item of the player's pack or sell basket with no flags (a quarter of its value); the card SHALL carry the item's description phrase. The item SHALL be the top-most icon under the pointer, as drawn. A press over the shown card SHALL be the card's own. The right release that follows either SHALL NOT close the trade window.

#### Scenario: A stock item's card
- **WHEN** the player right-clicks a shop copy of value 215 in the stock
- **THEN** its card opens with value 215 and its condition sentence, and the trade window stays open

#### Scenario: A basket item's card
- **WHEN** the player right-clicks a shop copy in the buy basket
- **THEN** its card opens with its whole value
