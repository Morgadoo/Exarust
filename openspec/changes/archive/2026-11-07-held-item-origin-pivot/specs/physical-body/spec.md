## ADDED Requirements

### Requirement: Held item origin pivot
A held item SHALL be tied to the part it hangs under (the holding hand, or a shield's forearm) by an origin pivot before its grip links: a pivot from the item's own origin to where that origin sits in the part's frame at the attach, with slack 0 and strength 1. Its grip links SHALL follow it, with every grip index counted past it.

#### Scenario: A sword turned about its grip line
- **WHEN** a held sword with the one-hander's two grip points is turned 40 degrees about the line through them
- **THEN** the grip stage brings its origin back under the hand, where two points alone leave it more than 5 cm off
