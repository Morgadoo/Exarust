## ADDED Requirements

### Requirement: Race inspection
`exa-tool races` SHALL print every race of the installation's `races.rdb`: its id, name, body model, movesets and health, its body parameters (support-pin factor, balance bonus, stance factor, arm relaxation, damping ratio) and its four gait rows.

#### Scenario: Retail races
- **WHEN** `exa-tool races` runs against a retail installation
- **THEN** 18 races are printed, Human with stance factor 0.5 and a walk thrust of 88.9 per second
