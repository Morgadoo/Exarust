## ADDED Requirements

### Requirement: Apparel tile offsets
An apparel paint job SHALL place its material-scale and damage-scale library tiles by four offsets drawn from the item's paint seed (the item body's word just before its appearance block), as a factory job draws them from its seed B.

#### Scenario: The braies
- **WHEN** the braies' item is painted
- **THEN** its tiling is the four offsets of its template's paint seed

### Requirement: Apparel wear by the stored material
An apparel layer's wear weight SHALL be the job's table entry for the layer's stored material byte, so a layer that draws its material from a random set SHALL take entry 0 (1.0); a random set that names no material SHALL leave material 0, painted as ClothBasic and reported as invalid.

#### Scenario: Velvet from a set
- **WHEN** a layer stores material 0 and random set 4, whose first material is Velvet
- **THEN** it paints Velvet with wear weight 1.0, not Velvet's 0.8
