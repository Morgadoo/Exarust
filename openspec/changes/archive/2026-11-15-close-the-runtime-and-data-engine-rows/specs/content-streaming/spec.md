## ADDED Requirements

### Requirement: Stream reset at quit
When the run is asked to end, the content stream SHALL empty every ring and its streamed list and stop after its current pass, so no request queued and not yet built is built afterwards.

#### Scenario: Requests queued at the quit
- **WHEN** an object request is being built, two more are queued and the run is asked to end
- **THEN** the queues are empty, the build in hand is dropped with its result and no later request is built
