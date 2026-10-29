# Spec Delta

## MODIFIED Requirements

### Requirement: Terrain set copy draws
A set copy SHALL draw one quad per destination target the features keep (colour always), sampling the source's same target at unit 0 with the vertex and texcoord at the same 0/1 corner. When the selection gives a non-blank holder's high set to another block and hands the holder a low set, the system SHALL record a copy from the holder's high set into that low set and, where the holder draws its finished high set, SHALL blit it before the taker's bake into the high set.

#### Scenario: Normal off
- **WHEN** a set without the normal feature is copied
- **THEN** no normal-target draw is planned

#### Scenario: The high pool runs out
- **WHEN** twenty-one blocks hold every high set and the focus moves onto a twenty-second block that wants one
- **THEN** the last holder takes a low set, state 0x500, and one copy is recorded from its old high set into that low set

#### Scenario: An unbaked holder
- **WHEN** the holder never drew its high set
- **THEN** no blit is queued and the holder's low set is rebaked from its layers
