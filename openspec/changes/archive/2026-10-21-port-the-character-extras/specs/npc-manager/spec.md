## ADDED Requirements

### Requirement: AI start-up
The AI start-up of a scene SHALL give the scene a `_Temp` node with flag bit 0x100000 when it has none, and SHALL independently start the AI manager only when none runs yet.

#### Scenario: A second scene
- **WHEN** a scene without `_Temp` starts while the manager already runs
- **THEN** the node is made and no second manager starts
