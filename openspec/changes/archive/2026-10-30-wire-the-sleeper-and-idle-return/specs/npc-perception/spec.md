# Spec Delta

## ADDED Requirements

### Requirement: Asleep is the dormant flag
The perception SHALL read a body as asleep when its state flags carry the dormant flag (0x10), whatever else they carry: an asleep observer's reach SHALL halve and an asleep observed body SHALL be weighed as lying. A sleeper and a knocked-out body both carry it.

#### Scenario: Sleeper and knocked-out body
- **WHEN** an observer with flags 0x10, and one with flags 0x30, look at a body with flags 0x30
- **THEN** both observers read themselves as asleep and the body as asleep
