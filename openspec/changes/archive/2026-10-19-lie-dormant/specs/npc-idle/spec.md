# Spec Delta

## ADDED Requirements

### Requirement: Sleep
A sleeper SHALL become dormant and set the dormant flag without clearing the others. Its tone and tone target SHALL become 0, it SHALL be a statue, and its gait SHALL be 2. The tick installed afterwards SHALL change nothing. A second sleep SHALL keep that flag and the dormant condition.

#### Scenario: A walking body lies dormant
- **WHEN** a walker whose condition is walking, whose flags are the shock stagger, whose tone is 1 and whose gait is 0 sleeps, its tick runs, and it sleeps again
- **THEN** the condition is dormant, the flags are the stagger and the dormant flag, the tone and the tone target are 0, it is a statue with gait 2, the tick leaves all of that as it was, and the second sleep keeps it
