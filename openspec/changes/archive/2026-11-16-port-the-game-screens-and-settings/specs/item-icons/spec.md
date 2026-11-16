## ADDED Requirements

### Requirement: Icon render variant
An icon SHALL be rendered with the cache variant's top two bits, the effect bit when the model flags have 0x2000, and the weapon bit when the item has a weapon section.

#### Scenario: A weapon's icon
- **WHEN** a weapon's icon is rendered
- **THEN** its render variant has the weapon bit and the sun takes the weapon preset
