# Spec Delta

## ADDED Requirements

### Requirement: Character preview bars
A character preview whose bars are switched on SHALL draw the HUD's health and focus bars at the bottom of the view: the health bar 50 GUI units left of the view's middle and 5 units above its bottom, the focus bar 52 units further right, both 48 units wide, the health bar with the bound record's two health shares (no floor on the lasting share) and the focus bar full. The arena hub's loadout form SHALL switch them on when it binds a person.

#### Scenario: Loadout preview
- **WHEN** `exarust grid --size 3200x2600 "play arenahub --open-hub-window loadout …"` runs
- **THEN** the manager's preview shows a full yellow health bar and a full blue focus bar at its bottom edge
