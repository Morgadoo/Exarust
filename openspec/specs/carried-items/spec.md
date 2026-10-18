# carried-items Specification

## Purpose
What a character wears, carries and keeps in a container: equipping by occupancy, the underwear rule, and moving an item between a container and a pack.

## Requirements

### Requirement: Equip by occupancy
Putting an item on SHALL take off every worn item whose occupancy shares a sub-part on the same layer of any bone. Displaced braies SHALL be dropped. Any other displaced item SHALL return to the pack. A human left with nothing over the groin SHALL wear braies again.

#### Scenario: Trousers over braies
- **WHEN** a human wearing braies puts on trousers that cover the groin, then takes the trousers off
- **THEN** the braies are not in the pack while the trousers are worn, and they are worn again once the trousers are off

#### Scenario: Another layer
- **WHEN** a shirt on a higher layer is worn under a coat that occupies that same layer
- **THEN** the shirt returns to the pack and the coat is worn with the braies

### Requirement: Container transfer
Moving an item SHALL take it out of the source list and append it to the destination. Moving an id the source does not hold SHALL leave both lists unchanged.

#### Scenario: Out of a chest
- **WHEN** a chest holds an item and a pack does not, and the item is moved to the pack
- **THEN** the chest no longer holds it and the pack does, and moving it again does nothing
