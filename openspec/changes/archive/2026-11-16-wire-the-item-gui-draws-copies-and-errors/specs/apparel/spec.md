## ADDED Requirements

### Requirement: Unknown procedural material
A paint draw for a material id outside 1 to 25 SHALL paint as ClothBasic, and the engine log SHALL show "Error: Invalid Procedural Material ID." at level 4 the first time each such id is met.

#### Scenario: The same unknown id twice
- **WHEN** the painter meets material 77 twice
- **THEN** the overlay shows the error once
