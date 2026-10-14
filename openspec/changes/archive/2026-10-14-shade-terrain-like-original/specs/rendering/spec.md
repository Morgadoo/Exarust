## ADDED Requirements

### Requirement: Vertical light shaping
The system SHALL shape lights flagged `0x200` (under mask `0x300`) vertically by their two extra values, as the original does, on surfaces and in the GI injection. Other lights are unshaped.

#### Scenario: Unshaped light
- **WHEN** a light's flags do not ask for shaping
- **THEN** its light falls off by distance alone
