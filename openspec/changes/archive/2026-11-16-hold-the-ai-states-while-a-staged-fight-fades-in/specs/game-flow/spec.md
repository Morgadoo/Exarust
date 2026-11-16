## ADDED Requirements

### Requirement: The practice staging sets the staging freeze
The practice start and the next practice fight SHALL end their staging with the staging freeze, before the practice hook; the step SHALL be posted for the next fixed step, where it sets the simulation's freeze.

#### Scenario: Start and next
- **WHEN** a practice starts and its next fight is staged, and the freeze step is applied and the fixed queue run
- **THEN** each staging holds the freeze step right before the practice hook, and the freeze is set only once the queue runs
