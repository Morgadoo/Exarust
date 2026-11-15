## ADDED Requirements

### Requirement: The locomotion push precedes the lift
While the body locomotes, each character tick SHALL add the drive × S² × the gait's push gain × the feet's lag to the carry before the lift servo, whatever the servo finds, with no support test of its own.

#### Scenario: A pelvis over its target
- **WHEN** a moving body's pelvis rides 20 cm over its target, so the servo gives no lift
- **THEN** its carry still takes the whole push
