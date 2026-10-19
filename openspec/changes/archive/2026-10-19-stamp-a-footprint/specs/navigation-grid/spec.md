# Spec Delta

## ADDED Requirements

### Requirement: Footprint
A footprint SHALL add 63 to its centre, 32 to the eight neighbours, and 16 to the twelve cells beyond, on the layer whose underside is within three below the height and whose floor is within twenty-five above. The outer five cells SHALL not be stamped. Removing it SHALL take the same amounts back. That centre SHALL make the obstacle test fire, and removing it SHALL clear the test.

#### Scenario: Sixty-three at the centre, then gone
- **WHEN** a footprint is stamped at a floor of height 1, removed, tried in the outer five cells, and stamped again on an upper floor at height 5
- **THEN** the centre is 63, a neighbour is 32, a cell beyond is 16, the obstacle test fires and then clears, the outer cell stays 0, and the upper floor's centre is 63 while the lower stays 0
