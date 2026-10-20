# Spec Delta

## ADDED Requirements

### Requirement: Parallel per-body work
The system SHALL run each body's own work in a tick (integration, links, level detection, level friction, post-step) in parallel across bodies, keeping the original's interleaving with the pair passes and pair friction for the bodies those touch, with results identical to stepping the bodies one at a time.

#### Scenario: Free bodies
- **WHEN** several bodies that touch no other body tick together
- **THEN** each ends the tick exactly where the interleaved, one-at-a-time path leaves it
