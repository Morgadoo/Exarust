## ADDED Requirements

### Requirement: Bodies offered to a cloth
The cloth stage SHALL offer a cloth only the body parts whose mesh box's world centre lies in the nine 600 cm grid cells around the cloth's own cell (its own cell, then the eight around it), before the sleep filter and the object pre-test; a part three cells away SHALL never be offered, however many there are. The offered parts SHALL be found from a bucketing of the level's parts built once a tick, not by scanning every part for each cloth.

#### Scenario: Near and far parts
- **WHEN** an awake cloth hangs in cell x 0 with one body part in its cell, one in the next cell and twenty three cells away
- **THEN** two parts are offered to it

#### Scenario: A part across the cell edge
- **WHEN** a cloth's centre lies at x 599 cm and a part's box centre at x 601 cm
- **THEN** the part is offered
