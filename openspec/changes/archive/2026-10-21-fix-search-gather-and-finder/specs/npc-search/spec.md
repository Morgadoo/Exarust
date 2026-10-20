## MODIFIED Requirements

### Requirement: Gathering
Gathering SHALL flood the 600 cm room grid from the anchor's cell through the four neighbours inside the border, flagged connected and closer than √11 cells. A container-like object SHALL be a candidate when searchable, not fixed, not searched and with an item record; a dead character's object when its pack is not searched. A character of a flooded cell whose object is container-like SHALL be weighed as that object, before the dead-character test. Each candidate SHALL be reserved.

#### Scenario: The flood's reach
- **WHEN** chests lie at the anchor's cell, three cells and one row off, three cells and two rows off, in a cell not connected, and fixed, searched, unsearchable or record-less ones, with a dead and a living character one row off
- **THEN** the anchor's chest, the one three cells and one row off and the dead character are gathered and reserved

#### Scenario: A character with a container-like object
- **WHEN** a living and a dead character of the anchor's cell each have a chest with an item record as their object, and the dead one's chest is searched
- **THEN** the living one's chest is gathered and reserved and the dead one's is not, though its pack is not searched
