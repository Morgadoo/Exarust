## ADDED Requirements

### Requirement: Parent skin shade
At place 10, 20 of size 100 by 80 and inset 5 the shade SHALL draw one rectangle from 15, 25 to 105, 95, with the colour taken from the place and alpha forced to 255. It SHALL count one prepare, two mode calls and one rectangle batch.

#### Scenario: The inset rectangle draws
- **WHEN** the shade is drawn at place 10, 20, size 100 by 80, inset 5, with colour 1, 2, 3, 4
- **THEN** the corners are (105, 25), (15, 25), (15, 95), (105, 95)
- **AND** the colour is 1, 2, 3, 255
- **AND** the counts are one prepare, modes 1 and 0x10, four vertices and four colour writes
