## ADDED Requirements

### Requirement: Retried bar cast
A retry of a held slot SHALL cast the picked power as a use does, and its answer SHALL renew the hold on "not now" or let it go otherwise; a retry that casts nothing SHALL let it go.

#### Scenario: Retry answered
- **WHEN** a retry starts the power
- **THEN** the hold is gone
