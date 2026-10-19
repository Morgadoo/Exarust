# Spec Delta

## ADDED Requirements

### Requirement: Torrent mix
While the phase is below 1, a torrent level SHALL be (2 minus the charge) times the charge times 0.4. Otherwise the level SHALL fall by 0.01, and below 0.02 it SHALL stop. Phase 1 SHALL ease the two channels toward 0.5 and 0.6 by 0.06. Phase 2 SHALL multiply both channels by 0.96.

#### Scenario: A full charge, an ease, and a stop
- **WHEN** a charge of 1 and of one half is stepped below phase 1, phase 1 eases channels that start at 0 while the level is 0.4, and phase 2 dims channels that start at 1 while the level is 0.4 and again while it is 0.03
- **THEN** the levels are 0.4 and 0.3, the eased channels are 0.03 and 0.036, the dimmed channels are 0.96, and the 0.03 level stops
