## ADDED Requirements

### Requirement: Parry point at the blade's parry height
The armed parry and the counter hold SHALL measure their point from the attacker's blade point at the parry height (the blade length less 20 cm out of the attacker's reach, less the reach left inside it, at least 0) on the blade's local axis, now and a tick ago, not from its tip.

#### Scenario: Out of reach
- **WHEN** the chests are 200 cm apart and the attacker's reach is 100 cm
- **THEN** the point starts at the blade's length less 20 cm along the blade

### Requirement: Shield plane link
A shield's armed grip SHALL make the shield's parry link a plane link to the parry point along the holder's frame forward with the grip's strength, and the parry's end SHALL disable it.

#### Scenario: A shield parry
- **WHEN** an armed defender behind a shield parries a blow
- **THEN** the shield's parry link holds the parry point as a plane link until the parry ends
