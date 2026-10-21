## ADDED Requirements

### Requirement: Painted layers
The system SHALL paint an item's appearance layers in order, all eight of them, skipping a layer without a mask and ending the list at the first layer whose mask the texture set lacks.

#### Scenario: Eight masks
- **WHEN** an appearance names masks 1 to 8
- **THEN** all eight layers are painted

#### Scenario: Missing mask
- **WHEN** the braies' item lists four masks and their texture set has two
- **THEN** only the layers before the first missing mask are painted
