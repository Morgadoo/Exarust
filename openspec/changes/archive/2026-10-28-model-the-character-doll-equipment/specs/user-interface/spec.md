## ADDED Requirements

### Requirement: Character view apparel drop
Apparel dropped onto the figure from another panel SHALL be worn with both characters rebuilt and SHALL charge the other pack its bulk whether or not the view is locked, without handing it the item, at a mind cost equal to its wear room.

#### Scenario: Apparel in a locked view
- **WHEN** apparel is dropped from another pack onto a locked view
- **THEN** that pack is charged its bulk and is not given the item

### Requirement: Character view weapon drop
A weapon or shield dropped onto the figure SHALL be attached on the doll and the shown character and, when the view is not locked, handed to the other pack marked held. A weapon SHALL cost twice its desirability less what it displaces, only with a second character shown. A drop back on the view's own panel SHALL only charge back the refunded cost.

#### Scenario: An unlocked weapon drop
- **WHEN** a weapon worth 5 displacing 1.25 is dropped on an unlocked view with a second character shown
- **THEN** both characters take it, the other pack is handed it and the cost is 7.5

### Requirement: Character view release
A cancelled release SHALL charge the refunded mind cost again and do nothing else. A locked view's drag of an item flagged 0x80000000 SHALL be released normally with the answer -1. A release of the shown item SHALL take it off and set the shown character's mark again without a rebuild.

#### Scenario: A locked flagged drag
- **WHEN** a locked view releases a dragged item flagged 0x80000000
- **THEN** no cost is charged, the item is taken off and the answer handed on is -1

### Requirement: Character view drag cost
A drag off the figure SHALL refund twice a weapon's desirability, else an apparel's protection value, reading the weapon section first.

#### Scenario: An item with both sections
- **WHEN** an item with apparel value 3 and weapon desirability 4 is dragged off the figure
- **THEN** the refund is 8
