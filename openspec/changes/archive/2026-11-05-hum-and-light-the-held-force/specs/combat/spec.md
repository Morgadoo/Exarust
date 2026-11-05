## ADDED Requirements

### Requirement: Striker hook on a landed blow
The system SHALL call the striker's hook on a landed blow with the contact and a strength: in a clash's and a block's branch after the struck behaviour's notice, with the impact at the hit; on flesh at the end of the damage routine, after the damage call, its sounds and the player's feedback, with the yellow damage before its 0.75. A turned blow on flesh SHALL NOT call it.

#### Scenario: A blow on a shield
- **WHEN** a striker's weapon meets a held shield
- **THEN** its hook is called with the impact at the hit
