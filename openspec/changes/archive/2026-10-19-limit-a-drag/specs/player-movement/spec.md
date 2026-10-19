# Spec Delta

## ADDED Requirements

### Requirement: Drag reach
A drag SHALL pull along the cursor direction by the nearest accepted hit, starting from 1000 cm. A hit at or beyond 1000 cm SHALL leave that reach. The grabbed box SHALL be refused when any placed corner is higher than 280 cm or lower than 1000 cm, and a corner on either limit SHALL be kept.

#### Scenario: A near hit and a high box
- **WHEN** hits at 1200 cm and 400 cm are taken along +X, and a box centred at 270 cm with 10 cm of half-height is tested, then the same centre with 20 cm of half-height
- **THEN** the pull is 400 cm along +X, the first box is held, and the second is refused
