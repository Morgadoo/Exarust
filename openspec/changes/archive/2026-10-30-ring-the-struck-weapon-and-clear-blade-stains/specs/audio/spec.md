## MODIFIED Requirements

### Requirement: Hit sounds
The system SHALL play a hit's rings at the contact point as the combat rules work them out: on a wound the striking weapon's set and each armour layer's set at their volumes and pitches, and a pain sound; on a clash the striking weapon and the struck weapon; on a block the striking weapon and the struck shield or torch.

#### Scenario: Sword hit
- **WHEN** a sword strike lands on a fighter's bare limb
- **THEN** the sword's impact, the flesh layer's `FlsCol` and a grunt play from the contact point

#### Scenario: Clash
- **WHEN** a sword clashes on a held weapon for the first time in its swing
- **THEN** the striking sword's ring and the struck weapon's set play from the contact point
