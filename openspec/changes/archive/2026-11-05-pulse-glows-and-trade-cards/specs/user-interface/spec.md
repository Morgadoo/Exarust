# Spec Delta

## ADDED Requirements

### Requirement: Trade window item cards
A right press over an item of the trade window's stock SHALL ask for that item's card with the flags `0x80000000` (the card then shows its whole value), and over an item of the player's pack with no flags (a quarter of its value); the card SHALL carry the item's description phrase. A press over the shown card SHALL be the card's own. The right release that follows either SHALL NOT close the trade window.

#### Scenario: A stock item's card
- **WHEN** the player right-clicks a shop copy of value 215 in the stock
- **THEN** its card opens with value 215 and its condition sentence, and the trade window stays open

### Requirement: Item card over the hub windows
The item card SHALL be drawn over the hub's windows: it is an upper-layer form (attributes `0x310F0`) and the trade window is on the normal layer (`0x830022`), so raising the window keeps it under the card.

#### Scenario: A bought item's card
- **WHEN** the scripted purchase opens the bought copy's card while the trade window is open
- **THEN** the card draws over the window
