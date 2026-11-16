## ADDED Requirements

### Requirement: Immediate push strain order
The immediate push of a body contact SHALL add to each side's strain the length of its push summed as (x² + z²) + y² before the square root.

#### Scenario: Orders that differ in the last bit
- **WHEN** a push of (−0.000490921608, −0.000453272893, −0.000319080515) m is added to a part's strain
- **THEN** the strain is the root of (x² + z²) + y², which differs from the root of (x² + y²) + z²
