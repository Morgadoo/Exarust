# Spec Delta

## ADDED Requirements

### Requirement: Painted apparel
The system SHALL colour worn apparel by painting each item's texture set from its shade map, layer masks and the appearance (material and palette colour per layer, quality, damage, dirt) of the object-database item that shows the model.

#### Scenario: Doublet
- **WHEN** a character wears `doublet_a`
- **THEN** its run-time materials show the cloth colours of the item "Doublet A" instead of the grey shade map

#### Scenario: Missing appearance
- **WHEN** no item shows a worn model
- **THEN** the model keeps its unpainted material
