# Spec Delta

## ADDED Requirements

### Requirement: Blows reach the struck behaviour
Every weapon contact dealt to a fighter SHALL tell the struck body's behaviour of the blow once: a clash on a held weapon as kind 1 and a block on a held shield as kind 2, first in their branch, and a wound on flesh as kind 0 after the damage, its sounds and the damage-taken notice. A blow the body turns SHALL tell it nothing. The struck body SHALL count as down when its condition is down, collapsing or dead at that point, a killing wound included.

#### Scenario: A wound, a clash, a block and a kill
- **WHEN** a sword wounds a standing fighter, then clashes on its weapon, then strikes its shield, then deals it a killing wound
- **THEN** its behaviour is told kind 0, 1 and 2 not down, then kind 0 down, and the striker's score rises by 3 for each wound

### Requirement: The AI's reaction to a blow
A character with a brain SHALL react to each blow it is told of in this order: the blow on a combatant, then the hit reaction's concern rise for the attacker, with the threat estimate over its nearby characters taken only when the attacker's contact is under the cap.

#### Scenario: A wound from the player beside a bystander
- **WHEN** a brain with courage 0.7 and confidence 0.4 and a neutral contact for the player is wounded by the player, a bystander standing 100 cm along the player's look within its reach plus 100 cm
- **THEN** its courage becomes `((1.4)·0.025 + 0.95)·0.7`, the player's swing is marked and offered its combat rating, and the player's contact's concern becomes 1.49 with its flags word 3

## MODIFIED Requirements

### Requirement: A blow on a combatant
A blow SHALL do nothing to a character that is down, dead or being removed. A wound SHALL lower its courage to `((confidence + 1)·0.025 + 0.95)·courage`, the sum in single precision and the rest in double. An attacker whose swing is not marked SHALL be marked, and its practice SHALL take the struck character's combat rating as its lesson when higher.

#### Scenario: Two hits
- **WHEN** a character with combat rating 30 is wounded by an attacker whose lesson is 10, then clashed by the same swing at rating 50, then wounded while down
- **THEN** its courage is lowered once, and the attacker's lesson is 30 and its swing is marked

### Requirement: The hit reaction's concern rise
A blow from an attacker the combatant has a contact for SHALL set that contact's flags word to 3 and SHALL raise its concern, when it is under 15, by the threat estimate plus 0.5, plus 0.5 more for a kind-1 hit or 0.25 for a kind 2, floored at zero and scaled by one less the contact's attitude times 0.01 in double precision, capped at 15. It SHALL run while the struck character is down or dead.

#### Scenario: A kind-1 hit from a half-friendly attacker
- **WHEN** a contact with attitude 50 and concern 1 is hit with kind 1 and threat 0
- **THEN** its concern becomes 1.5, its concerned event is raised and its flags word is 3

#### Scenario: A NaN concern
- **WHEN** the contact's concern is NaN
- **THEN** the hit adds nothing

#### Scenario: At the cap
- **WHEN** the contact's concern is 15
- **THEN** its flags word becomes 3, its concern stays and no threat estimate is taken
