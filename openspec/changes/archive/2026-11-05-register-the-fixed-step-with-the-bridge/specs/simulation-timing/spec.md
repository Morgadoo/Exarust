## ADDED Requirements

### Requirement: Whole fixed step bridged
With the bridge on, every data type the moved fixed step's systems and run conditions touch SHALL be bridged, and no moved system SHALL be skipped for missing data.

#### Scenario: Three levels
- **WHEN** the arena duel, exanima01 and exanima06 run with the bridge
- **THEN** the bridge reports no unbridged type and no system skipped

### Requirement: Copy cost reported
The bridge SHALL report the time its copies take per step and the costliest types on each side.

#### Scenario: A level running
- **WHEN** a level runs 1000 steps with the bridge
- **THEN** the log gives the copy time per step and each side's costliest types
