# Spec Delta

## ADDED Requirements

### Requirement: Body coverage
The system SHALL hide the body parts that worn apparel covers, as given by each item's per-slot flags.

#### Scenario: Gambeson
- **WHEN** a character wears a gambeson, trousers and boots
- **THEN** no skin shows through them on the torso, arms, legs or feet
