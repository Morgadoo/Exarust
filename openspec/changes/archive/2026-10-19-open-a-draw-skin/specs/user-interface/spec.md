## ADDED Requirements

### Requirement: Draw skin
Opening a skin with a unit SHALL store twice that unit, the unit, and twice the unit twice more. It SHALL store the pads 8, 4, 8 and 8. Its three colours SHALL each be 255, 250, 230, 100. Its mark SHALL be 1. Argument 1 SHALL count an allocation, and a failed allocation SHALL return nothing. A non-zero argument SHALL count the call after the fill. Argument 0 SHALL not. The base fill and that later call are counts, not their bodies.

#### Scenario: Unit 5 stores the spans and the colour
- **WHEN** a skin is opened with unit 5 and argument 0, and again with argument 1 after a successful allocation, and again with argument 1 after a failed allocation
- **THEN** the first stores spans 10, 5, 10 and 10 and the colour three times without the later call, the second counts the allocation and the later call, and the third returns nothing
