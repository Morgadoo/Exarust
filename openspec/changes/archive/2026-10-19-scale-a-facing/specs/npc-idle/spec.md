# Spec Delta

## ADDED Requirements

### Requirement: A dead body is knocked out
A dead body SHALL be knocked out at the current clock with no delay, down, with the dormant and waking flags. A walking body SHALL stay walking.

#### Scenario: Dead at clock 50, walking stays
- **WHEN** a dead body is entered at clock 50, and a walking body is entered at clock 90
- **THEN** the dead body wakes at 50, is down, and has the dormant and waking flags, and the walking body stays walking
