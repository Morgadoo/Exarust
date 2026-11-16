## ADDED Requirements

### Requirement: A removed door's creak voice
A door leaf that leaves the world (its level unloaded, its entity despawned) SHALL have the creak voice it held freed, as the door's destructor releases it.

#### Scenario: Despawned mid-creak
- **WHEN** a leaf's creak voice plays and the leaf is despawned
- **THEN** after the next mechanism sounds pass no voice is held
