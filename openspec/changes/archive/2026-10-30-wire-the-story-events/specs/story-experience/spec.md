# Spec Delta

## ADDED Requirements

### Requirement: Technique word on the body
The level start SHALL put the technique word the player's skills name on the player's body, and a rise that changes it SHALL put the new word there; the body's caster SHALL take the word as its technique bits and its fighter SHALL gain the perks the record's techniques give, keeping those it had. The live hooks SHALL read and write the record the player's body carries; a story player SHALL always carry one.

#### Scenario: A body takes its word
- **WHEN** a body whose record funds close combat's first technique takes its technique word
- **THEN** its caster's technique bits equal the word and its fighter has the record's perks
