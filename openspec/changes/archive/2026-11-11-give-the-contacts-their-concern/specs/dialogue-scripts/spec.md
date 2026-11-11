## ADDED Requirements

### Requirement: A character's conversations end
On request the dialogue engine SHALL end every live conversation in which the given character is the owner or the partner, as the player's leave ends its own.

#### Scenario: A fleeing NPC
- **WHEN** an NPC that is speaking to the player starts to flee
- **THEN** that conversation ends and its panel closes
