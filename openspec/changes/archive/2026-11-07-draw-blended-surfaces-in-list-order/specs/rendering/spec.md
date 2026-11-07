## ADDED Requirements

### Requirement: Blended list order on the transparent phase
Every blended surface material SHALL carry its render state (decal bit, blend layer, opacity, build order), and the transparent phase SHALL draw the decal states first in build order, then the blended states in the blended list's layer and opacity selection order, then surfaces without a state by view depth.

#### Scenario: A faint and a dense glass
- **WHEN** a faint glass is built before a dense glass on the same layer
- **THEN** the dense glass is drawn first, whatever their distances
