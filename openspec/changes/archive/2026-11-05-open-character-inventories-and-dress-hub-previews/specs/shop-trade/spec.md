# Spec Delta

## ADDED Requirements

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
