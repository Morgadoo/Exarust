## ADDED Requirements

### Requirement: Blend layer
A render state's blend layer SHALL be bits 4–7 of its blend word.

#### Scenario: High and low bits ignored
- **WHEN** the blend word is 0x0212
- **THEN** the layer is 1
