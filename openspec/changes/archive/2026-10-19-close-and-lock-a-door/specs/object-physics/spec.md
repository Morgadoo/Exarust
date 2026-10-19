# Spec Delta

## ADDED Requirements

### Requirement: Close and lock
A motorised hinge SHALL set its fraction to 1 and drive toward shut for 300 ticks, or 100 when fast. A hinge that is not motorised SHALL not start that drive. A sliding leaf that is not latched SHALL play the lock sound, and a fast one SHALL drive toward shut for 100 ticks. A leaf that is already shut SHALL become latched.

#### Scenario: A motor, a plain hinge, and a fast slider
- **WHEN** a motorised hinge at fraction 0.2 is closed, a fast motorised hinge is closed, a shut hinge that is not motorised is closed, an unlatched sliding leaf is closed, and a shut fast sliding leaf is closed
- **THEN** the motorised hinge is at fraction 1 for 300 ticks and is not latched, the fast hinge drives for 100 ticks, the plain hinge stays idle and becomes latched, the sliding leaf plays the lock sound and stays unlatched, and the fast slider drives for 100 ticks and becomes latched
