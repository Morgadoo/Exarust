## ADDED Requirements

### Requirement: Armed parry trace
At debug level, the armed parry SHALL log the tick an item's parry link goes from disabled to held. The log line SHALL name the item, the parry point and the link's strength.

#### Scenario: A sword duel
- **WHEN** an armed fighter parries with its sword
- **THEN** the log shows `armed parry:` with the sword's link held toward the parry point
