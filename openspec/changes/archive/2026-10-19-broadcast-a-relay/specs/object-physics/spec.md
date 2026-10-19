# Spec Delta

## ADDED Requirements

### Requirement: Relay broadcast
A broadcast SHALL signal every tagged object whose door link equals the relay, including the source. A door link that does not match SHALL be skipped even when the operative target matches. Without a door link, an operative SHALL be signalled when its target equals the relay, except the source itself. An untagged id SHALL be skipped. The value and the relay id SHALL be forwarded in list order.

#### Scenario: The source door and one other operative
- **WHEN** the list holds an untagged listener, a source door linked to the relay, a door linked elsewhere that also listens, the source as an operative listener, another operative listener, and an operative listening elsewhere, and the value is 5
- **THEN** only the source door and the other operative are signalled, in that order, with the relay as the source and the value 5
