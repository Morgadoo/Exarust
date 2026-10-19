## ADDED Requirements

### Requirement: Parent bevel arms
`draw_skin_bevel` SHALL take two booleans and select the raised arm when the first is true, the pressed arm when the first is false and the second true, and the idle arm otherwise. It SHALL delegate the fills, six border lines and four outline lines to the shared bevel implementation.

#### Scenario: The first arm wins
- **WHEN** the first boolean is true and the second is true
- **THEN** the raised arm is used
- **WHEN** the first boolean is false and the second is true
- **THEN** the pressed arm is used
- **WHEN** both are false
- **THEN** the idle arm is used
