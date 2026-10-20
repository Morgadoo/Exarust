## ADDED Requirements

### Requirement: Kinesis scale
The kinesis scale SHALL be (1/speed + 0.015)·0.5.

#### Scenario: Two speeds
- **WHEN** the speed is 1
- **THEN** the scale is 0.5075
- **WHEN** the speed is 2
- **THEN** the scale is 0.2575

### Requirement: Kinesis activation
The effect SHALL move to removal when the event carries bit 1 or 3. Removal SHALL release a held target and return 2. The active state SHALL hook the owner when it accepts, raise the target's flag and register the callback.

#### Scenario: Active and removed
- **WHEN** the effect is active and the owner accepts the hook
- **THEN** the hook, target flag and registration are set and the result is 0
- **WHEN** an event with bit 3 arrives
- **THEN** the state is removal, the target is released and the result is 2
