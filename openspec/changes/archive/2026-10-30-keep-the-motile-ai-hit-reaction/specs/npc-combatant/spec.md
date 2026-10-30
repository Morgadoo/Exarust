# Spec Delta

## ADDED Requirements

### Requirement: The hit reaction's concern rise
A blow from an attacker the combatant has a contact for SHALL raise that contact's concern, when it is under 15, by the threat estimate plus 0.5, plus 0.5 more for a kind-1 hit or 0.25 for a kind 2, floored at zero and scaled by one less the contact's attitude times 0.01 in double precision, capped at 15.

#### Scenario: A kind-1 hit from a half-friendly attacker
- **WHEN** a contact with attitude 50 and concern 1 is hit with kind 1 and threat 0
- **THEN** its concern becomes 1.5 and its concerned event is raised

#### Scenario: A NaN concern
- **WHEN** the contact's concern is NaN
- **THEN** the hit adds nothing

### Requirement: The hit reaction's threat estimate
The threat estimate SHALL be one less the largest share `dot / d²` among the observer's nearby living bodies other than the attacker, where `d` is the body's aim point less the attacker's position, `d²` lies between 1 cm² and the square of the attacker's radius plus 100 cm, and the dot with the attacker's look is not negative.

#### Scenario: One body ahead of the attacker
- **WHEN** the only nearby body's aim point lies 100 cm along the attacker's look and the attacker's radius is 40 cm
- **THEN** the threat estimate is 0.99
