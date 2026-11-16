## ADDED Requirements

### Requirement: Door drive after integration
A hinged door's drive SHALL be measured from the leaf as the last tick left it and SHALL be pushed onto the stop point by its weights right after the leaf's next integration, before its link passes, so that the integration does not carry the push. A leaf that sleeps through that tick SHALL drop the push.

#### Scenario: A motorised open
- **WHEN** a woken hinged leaf with a motorised open command takes a door step and then its next integration
- **THEN** its particles equal the integrated particles plus the drive's push, not the integration of the pushed particles

#### Scenario: A sleeping leaf
- **WHEN** a leaf with a queued drive is asleep at its next tick
- **THEN** the queued drive is dropped
