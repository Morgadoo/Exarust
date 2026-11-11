## ADDED Requirements

### Requirement: Portal use step
The level exit's portal-use step SHALL run the portal use on the player's body (its dissolve burst and `portaluse01`) when the flow applies it, before the transition opens.

#### Scenario: A portal exit
- **WHEN** the player crosses a level exit zone with the portal bit
- **THEN** the portal use runs on the player's body and the long transition starts
