## ADDED Requirements

### Requirement: Jolt soft stop
An AI body whose last balance tick was jolted while it moved SHALL be soft-stopped before its states run: its route ends and its request takes the stopping bit.

#### Scenario: A jolted walker
- **WHEN** a route-following body's balance tick reports a jolt
- **THEN** at the next AI tick its route no longer follows and its request is stopping
